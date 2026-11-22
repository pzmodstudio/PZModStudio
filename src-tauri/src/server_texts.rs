//! What a server says to its players and to Discord, in the seven languages of the app: the
//! in-game restart warnings (`servermsg` over RCON), the Discord messages (webhook) and the answers
//! of the Discord commands. The language is the app's when the settings were saved; one that is
//! not here reads English.

/// The languages, in the order of each text below.
pub const LANGUAGES: [&str; 7] = ["en", "es", "pt-BR", "fr", "de", "ru", "zh-CN"];

/// One of LANGUAGES from what was saved ("es", "es-AR", "pt", "zh"...), English when unknown.
pub fn language(code: &str) -> &'static str {
    let lower = code.trim().to_lowercase();
    if let Some(l) = LANGUAGES.iter().find(|l| l.to_lowercase() == lower) {
        return l;
    }
    let base = lower.split(['-', '_']).next().unwrap_or("");
    LANGUAGES.iter().find(|l| l.to_lowercase().split('-').next() == Some(base)).copied().unwrap_or("en")
}

fn index(code: &str) -> usize {
    let l = language(code);
    LANGUAGES.iter().position(|x| *x == l).unwrap_or(0)
}

/// key -> [en, es, pt-BR, fr, de, ru, zh-CN]; `{0}`, `{1}`... are filled in by `text`.
const TABLE: &[(&str, [&str; 7])] = &[
    // ---- in the game (one line, no quotes)
    ("game.warn_one", [
        "Server restart in 1 minute{0}. Find a safe place.",
        "Reinicio del servidor en 1 minuto{0}. Buscá un lugar seguro.",
        "Reinício do servidor em 1 minuto{0}. Procure um lugar seguro.",
        "Redémarrage du serveur dans 1 minute{0}. Mettez-vous à l'abri.",
        "Server-Neustart in 1 Minute{0}. Such dir einen sicheren Ort.",
        "Перезапуск сервера через 1 минуту{0}. Найдите безопасное место.",
        "服务器将在 1 分钟后重启{0}。请找个安全的地方。",
    ]),
    ("game.warn", [
        "Server restart in {0} minutes{1}.",
        "Reinicio del servidor en {0} minutos{1}.",
        "Reinício do servidor em {0} minutos{1}.",
        "Redémarrage du serveur dans {0} minutes{1}.",
        "Server-Neustart in {0} Minuten{1}.",
        "Перезапуск сервера через {0} мин.{1}",
        "服务器将在 {0} 分钟后重启{1}。",
    ]),
    ("game.why_mod_update", [" (a mod was updated)", " (se actualizó un mod)", " (um mod foi atualizado)", " (un mod a été mis à jour)", " (eine Mod wurde aktualisiert)", " (обновился мод)", "（有模组更新）"]),
    ("game.why_schedule", [" (scheduled)", " (programado)", " (programado)", " (programmé)", " (geplant)", " (по расписанию)", "（计划内）"]),
    ("game.now", [
        "Server restarting now: the world is being saved. Back in a few minutes.",
        "El servidor se reinicia ahora: se guarda el mundo. Volvé en unos minutos.",
        "O servidor está reiniciando agora: o mundo está sendo salvo. Volte em alguns minutos.",
        "Le serveur redémarre maintenant : le monde est sauvegardé. De retour dans quelques minutes.",
        "Der Server startet jetzt neu: die Welt wird gespeichert. In ein paar Minuten geht es weiter.",
        "Сервер перезапускается: мир сохраняется. Вернитесь через несколько минут.",
        "服务器正在重启：正在保存世界。几分钟后再回来。",
    ]),
    ("game.cancel", [
        "The server restart was called off.",
        "Se canceló el reinicio del servidor.",
        "O reinício do servidor foi cancelado.",
        "Le redémarrage du serveur est annulé.",
        "Der Server-Neustart wurde abgesagt.",
        "Перезапуск сервера отменён.",
        "服务器重启已取消。",
    ]),
    // ---- Discord, the server's news
    ("discord.test", [
        "✅ PZ Mod Studio is connected: news of **{0}** will show here.",
        "✅ PZ Mod Studio está conectado: las novedades de **{0}** se van a ver acá.",
        "✅ O PZ Mod Studio está conectado: as novidades de **{0}** vão aparecer aqui.",
        "✅ PZ Mod Studio est connecté : les nouvelles de **{0}** s'afficheront ici.",
        "✅ PZ Mod Studio ist verbunden: Neuigkeiten von **{0}** erscheinen hier.",
        "✅ PZ Mod Studio подключён: новости **{0}** будут появляться здесь.",
        "✅ PZ Mod Studio 已连接：**{0}** 的动态会显示在这里。",
    ]),
    ("discord.up", ["🟢 **{0}** is up.", "🟢 **{0}** está en línea.", "🟢 **{0}** está online.", "🟢 **{0}** est en ligne.", "🟢 **{0}** ist online.", "🟢 **{0}** запущен.", "🟢 **{0}** 已上线。"]),
    ("discord.restarted", [
        "🔄 **{0}** restarted and is up again.",
        "🔄 **{0}** se reinició y está en línea de nuevo.",
        "🔄 **{0}** reiniciou e está online de novo.",
        "🔄 **{0}** a redémarré et est de nouveau en ligne.",
        "🔄 **{0}** wurde neu gestartet und ist wieder online.",
        "🔄 **{0}** перезапущен и снова работает.",
        "🔄 **{0}** 已重启，重新上线。",
    ]),
    ("discord.stopping", [
        "🔴 **{0}** was stopped from PZ Mod Studio.",
        "🔴 **{0}** se detuvo desde PZ Mod Studio.",
        "🔴 **{0}** foi parado pelo PZ Mod Studio.",
        "🔴 **{0}** a été arrêté depuis PZ Mod Studio.",
        "🔴 **{0}** wurde aus PZ Mod Studio gestoppt.",
        "🔴 **{0}** остановлен из PZ Mod Studio.",
        "🔴 **{0}** 已从 PZ Mod Studio 停止。",
    ]),
    ("discord.crashed", [
        "⚠️ **{0}** stopped without being asked (a crash, or closed outside PZ Mod Studio).",
        "⚠️ **{0}** se detuvo sin que se lo pidieran (un cierre inesperado, o se cerró fuera de PZ Mod Studio).",
        "⚠️ **{0}** parou sem ninguém pedir (uma queda, ou foi fechado fora do PZ Mod Studio).",
        "⚠️ **{0}** s'est arrêté sans qu'on le demande (un plantage, ou fermé en dehors de PZ Mod Studio).",
        "⚠️ **{0}** hat ungefragt angehalten (ein Absturz, oder außerhalb von PZ Mod Studio geschlossen).",
        "⚠️ **{0}** остановился сам (сбой или закрыт не из PZ Mod Studio).",
        "⚠️ **{0}** 意外停止了（崩溃，或在 PZ Mod Studio 之外被关闭）。",
    ]),
    ("discord.joined", ["➡️ {0} joined ({1} online).", "➡️ Entró {0} ({1} en línea).", "➡️ {0} entrou ({1} online).", "➡️ {0} est arrivé ({1} en ligne).", "➡️ {0} ist beigetreten ({1} online).", "➡️ Зашёл {0} (в сети: {1}).", "➡️ {0} 加入了（在线 {1} 人）。"]),
    ("discord.left", ["⬅️ {0} left ({1} online).", "⬅️ Salió {0} ({1} en línea).", "⬅️ {0} saiu ({1} online).", "⬅️ {0} est parti ({1} en ligne).", "⬅️ {0} ist gegangen ({1} online).", "⬅️ Вышел {0} (в сети: {1}).", "⬅️ {0} 离开了（在线 {1} 人）。"]),
    ("discord.updates", [
        "📦 {0} of **{1}**'s Workshop mods were updated since it started: {2}. Players who have the new version cannot join until it restarts.",
        "📦 {0} mod(s) del Workshop de **{1}** se actualizaron desde que arrancó: {2}. Quien tenga la versión nueva no puede entrar hasta que se reinicie.",
        "📦 Mods da Oficina de **{1}** atualizados desde que ele iniciou ({0}): {2}. Quem tem a versão nova não consegue entrar até ele reiniciar.",
        "📦 Mods du Workshop de **{1}** mis à jour depuis son démarrage ({0}) : {2}. Les joueurs qui ont la nouvelle version ne peuvent pas entrer avant son redémarrage.",
        "📦 Seit dem Start aktualisierte Workshop-Mods von **{1}** ({0}): {2}. Wer die neue Version hat, kann erst nach einem Neustart beitreten.",
        "📦 Моды Мастерской на **{1}**, обновлённые после запуска ({0}): {2}. Игроки с новой версией не смогут зайти до перезапуска.",
        "📦 **{1}** 启动后有创意工坊模组更新（{0} 个）：{2}。拥有新版本的玩家要等它重启后才能加入。",
    ]),
    ("discord.restartWarn", [
        "🔄 **{0}** restarts in {1} min{2}: players are warned in the game.",
        "🔄 **{0}** se reinicia en {1} min{2}: los jugadores reciben el aviso en el juego.",
        "🔄 **{0}** reinicia em {1} min{2}: os jogadores são avisados no jogo.",
        "🔄 **{0}** redémarre dans {1} min{2} : les joueurs sont prévenus dans le jeu.",
        "🔄 **{0}** startet in {1} Min. neu{2}: die Spieler werden im Spiel gewarnt.",
        "🔄 **{0}** перезапустится через {1} мин.{2}: игроки предупреждены в игре.",
        "🔄 **{0}** 将在 {1} 分钟后重启{2}：已在游戏中提醒玩家。",
    ]),
    ("discord.restartCancelled", [
        "🚫 The restart of **{0}** was called off.",
        "🚫 Se canceló el reinicio de **{0}**.",
        "🚫 O reinício de **{0}** foi cancelado.",
        "🚫 Le redémarrage de **{0}** est annulé.",
        "🚫 Der Neustart von **{0}** wurde abgesagt.",
        "🚫 Перезапуск **{0}** отменён.",
        "🚫 **{0}** 的重启已取消。",
    ]),
    ("status.nobody", ["nobody on", "nadie conectado", "ninguém online", "personne en ligne", "niemand online", "никого нет", "无人在线"]),
    ("status.online", ["{0} online: {1}", "{0} en línea: {1}", "{0} online: {1}", "{0} en ligne : {1}", "{0} online: {1}", "в сети {0}: {1}", "{0} 人在线：{1}"]),
    ("status.up", ["🟢 **{0}** up since {1} · {2}", "🟢 **{0}** en línea desde {1} · {2}", "🟢 **{0}** online desde {1} · {2}", "🟢 **{0}** en ligne depuis {1} · {2}", "🟢 **{0}** online seit {1} · {2}", "🟢 **{0}** работает с {1} · {2}", "🟢 **{0}** 自 {1} 起在线 · {2}"]),
    ("status.down", ["🔴 **{0}** down since {1}", "🔴 **{0}** apagado desde {1}", "🔴 **{0}** desligado desde {1}", "🔴 **{0}** arrêté depuis {1}", "🔴 **{0}** offline seit {1}", "🔴 **{0}** выключен с {1}", "🔴 **{0}** 自 {1} 起离线"]),
    ("names.and", [" and ", " y ", " e ", " et ", " und ", " и ", "和"]),
    ("names.more", ["{0} more", "{0} más", "mais {0}", "{0} de plus", "{0} weitere", "ещё {0}", "另外 {0} 人"]),
    // ---- Discord, the answers to the commands
    ("cmd.help", [
        "Commands: `!status`, `!players`, `!restart [minutes]` (1-60, default 5), `!restart cancel`, `!save`, `!msg <text>`.",
        "Comandos: `!estado`, `!jugadores`, `!reiniciar [minutos]` (1-60, 5 si no decís), `!reiniciar cancelar`, `!guardar`, `!mensaje <texto>`. También en inglés: `!status`, `!players`, `!restart`, `!save`, `!msg`.",
        "Comandos: `!status`, `!players`, `!restart [minutos]` (1-60, 5 se não disser), `!restart cancel`, `!save`, `!msg <texto>`.",
        "Commandes : `!status`, `!players`, `!restart [minutes]` (1-60, 5 par défaut), `!restart cancel`, `!save`, `!msg <texte>`.",
        "Befehle: `!status`, `!players`, `!restart [Minuten]` (1-60, Standard 5), `!restart cancel`, `!save`, `!msg <Text>`.",
        "Команды: `!status`, `!players`, `!restart [минуты]` (1-60, по умолчанию 5), `!restart cancel`, `!save`, `!msg <текст>`.",
        "命令：`!status`、`!players`、`!restart [分钟]`（1-60，默认 5）、`!restart cancel`、`!save`、`!msg <文本>`。",
    ]),
    ("cmd.noRole", [
        "Only members with the role **{0}** can use these commands.",
        "Solo los miembros con el rol **{0}** pueden usar estos comandos.",
        "Só membros com o cargo **{0}** podem usar estes comandos.",
        "Seuls les membres ayant le rôle **{0}** peuvent utiliser ces commandes.",
        "Nur Mitglieder mit der Rolle **{0}** können diese Befehle nutzen.",
        "Эти команды доступны только участникам с ролью **{0}**.",
        "只有拥有 **{0}** 身份组的成员才能使用这些命令。",
    ]),
    ("cmd.down", ["🔴 **{0}** is not running.", "🔴 **{0}** no está en línea.", "🔴 **{0}** não está rodando.", "🔴 **{0}** ne tourne pas.", "🔴 **{0}** läuft nicht.", "🔴 **{0}** не запущен.", "🔴 **{0}** 没有运行。"]),
    ("cmd.up", [
        "🟢 **{0}** is up, {1} player(s) online.",
        "🟢 **{0}** está en línea, {1} jugador(es) conectado(s).",
        "🟢 **{0}** está online, jogadores conectados: {1}.",
        "🟢 **{0}** est en ligne, joueurs connectés : {1}.",
        "🟢 **{0}** ist online, Spieler online: {1}.",
        "🟢 **{0}** работает, игроков в сети: {1}.",
        "🟢 **{0}** 在线，{1} 名玩家在线。",
    ]),
    ("cmd.players", ["Online on **{0}**: {1}", "En línea en **{0}**: {1}", "Online em **{0}**: {1}", "En ligne sur **{0}** : {1}", "Online auf **{0}**: {1}", "В сети на **{0}**: {1}", "**{0}** 在线：{1}"]),
    ("cmd.nobody", ["Nobody is on **{0}**.", "No hay nadie en **{0}**.", "Não há ninguém em **{0}**.", "Personne sur **{0}**.", "Niemand ist auf **{0}**.", "На **{0}** никого нет.", "**{0}** 上没有人。"]),
    ("cmd.restartAsked", [
        "🔄 **{0}** restarts in {1} minute(s); players are warned in the game. `!restart cancel` stops it.",
        "🔄 **{0}** se reinicia en {1} minuto(s); los jugadores reciben el aviso en el juego. `!reiniciar cancelar` lo detiene.",
        "🔄 **{0}** reinicia em {1} min; os jogadores são avisados no jogo. `!restart cancel` cancela.",
        "🔄 **{0}** redémarre dans {1} min ; les joueurs sont prévenus dans le jeu. `!restart cancel` l'annule.",
        "🔄 **{0}** startet in {1} Min. neu; die Spieler werden im Spiel gewarnt. `!restart cancel` bricht ab.",
        "🔄 **{0}** перезапустится через {1} мин.; игроки предупреждены в игре. `!restart cancel` отменит.",
        "🔄 **{0}** 将在 {1} 分钟后重启；已在游戏中提醒玩家。`!restart cancel` 可取消。",
    ]),
    ("cmd.restartCancelled", ["The restart of **{0}** was cancelled.", "Se canceló el reinicio de **{0}**.", "O reinício de **{0}** foi cancelado.", "Le redémarrage de **{0}** est annulé.", "Der Neustart von **{0}** wurde abgesagt.", "Перезапуск **{0}** отменён.", "**{0}** 的重启已取消。"]),
    ("cmd.noRestart", ["No restart is waiting.", "No hay ningún reinicio esperando.", "Não há nenhum reinício pendente.", "Aucun redémarrage en attente.", "Kein Neustart steht an.", "Перезапуск не запланирован.", "没有待执行的重启。"]),
    ("cmd.saved", ["💾 **{0}** saved the world.", "💾 **{0}** guardó el mundo.", "💾 **{0}** salvou o mundo.", "💾 **{0}** a sauvegardé le monde.", "💾 **{0}** hat die Welt gespeichert.", "💾 **{0}** сохранил мир.", "💾 **{0}** 已保存世界。"]),
    ("cmd.sent", ["Said in the game.", "Dicho en el juego.", "Dito no jogo.", "Dit dans le jeu.", "Im Spiel gesagt.", "Сказано в игре.", "已在游戏中发送。"]),
    ("cmd.noRcon", [
        "⚠️ **{0}** has no RCON password, so the Studio cannot talk to it. Stop it and start it again from the Studio: it sets one.",
        "⚠️ **{0}** no tiene contraseña RCON, así que el Studio no puede hablarle. Detenelo y volvé a iniciarlo desde el Studio: le pone una.",
        "⚠️ **{0}** não tem senha RCON, então o Studio não consegue falar com ele. Pare e inicie de novo pelo Studio: ele define uma.",
        "⚠️ **{0}** n'a pas de mot de passe RCON, le Studio ne peut donc pas lui parler. Arrêtez-le et relancez-le depuis le Studio : il en définit un.",
        "⚠️ **{0}** hat kein RCON-Passwort, also kann das Studio nicht mit ihm reden. Stopp ihn und starte ihn aus dem Studio neu: es setzt eins.",
        "⚠️ У **{0}** нет пароля RCON, поэтому Studio не может с ним связаться. Остановите и запустите его из Studio: он задаст пароль.",
        "⚠️ **{0}** 没有 RCON 密码，Studio 无法与它通信。请停止它并从 Studio 重新启动：Studio 会设置一个。",
    ]),
    ("cmd.failed", ["⚠️ It did not work: {0}", "⚠️ No funcionó: {0}", "⚠️ Não funcionou: {0}", "⚠️ Ça n'a pas marché : {0}", "⚠️ Hat nicht geklappt: {0}", "⚠️ Не получилось: {0}", "⚠️ 没有成功：{0}"]),
];

/// The text of `key` in that language with `{0}`, `{1}`... filled in; the key itself when unknown.
pub fn text(code: &str, key: &str, args: &[&str]) -> String {
    let i = index(code);
    let mut out = TABLE.iter().find(|(k, _)| *k == key).map(|(_, t)| t[i].to_string()).unwrap_or_else(|| key.to_string());
    for (n, a) in args.iter().enumerate() {
        out = out.replace(&format!("{{{}}}", n), a);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_text_has_its_placeholders_in_every_language() {
        for (key, texts) in TABLE {
            let count = |s: &str| (0..4).filter(|n| s.contains(&format!("{{{}}}", n))).count();
            for (lang, t) in LANGUAGES.iter().zip(texts.iter()) {
                assert!(!t.trim().is_empty() || key.starts_with("names."), "{key} ({lang}) is empty");
                assert_eq!(count(t), count(texts[0]), "{key} ({lang}) has other placeholders than English");
                if key.starts_with("game.") || key.starts_with("cmd.game") {
                    assert!(!t.contains('"') && !t.contains('\\') && !t.contains('\n'), "{key} ({lang}) must fit in servermsg");
                }
            }
        }
    }

    #[test]
    fn a_saved_language_is_one_of_the_seven() {
        assert_eq!(language("es-AR"), "es");
        assert_eq!(language("pt"), "pt-BR");
        assert_eq!(language("zh"), "zh-CN");
        assert_eq!(language("ja"), "en");
        assert_eq!(text("ru", "cmd.saved", &["Coop"]), "💾 **Coop** сохранил мир.");
        assert_eq!(text("xx", "cmd.saved", &["Coop"]), "💾 **Coop** saved the world.");
    }
}
