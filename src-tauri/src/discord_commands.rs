//! Commands from Discord, while PZ Mod Studio runs: `!status`, `!players`, `!restart [minutes]`,
//! `!restart cancel`, `!save`, `!msg <text>`, `!help`, written in one channel by members with one
//! role. Off unless the player turns them on.
//!
//! They use the same bot as the game's own chat (its token is in the server's `.ini`), so there
//! is nothing more to make in Discord. There is no gateway connection: the channel is read over
//! Discord's REST API every few seconds (as PZMM does), waiting as long as a rate limit asks and
//! longer after errors. Channel and role are names, as in Discord; their ids are looked up once.
//! Replies never ping anyone. A member's roles are looked up once every few minutes.

use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

const API: &str = "https://discord.com/api/v10";
const EVERY: Duration = Duration::from_secs(5);
const MAX_BACKOFF_SECS: u64 = 300;
const ROLES_FOR_SECS: u64 = 300;
const RESTART_DEFAULT_MIN: u32 = 5;
const RESTART_MAX_MIN: u32 = 60;

#[derive(Debug, Clone, PartialEq)]
pub enum Command {
    Help,
    Status,
    Players,
    Restart(u32),
    RestartCancel,
    Save,
    Msg(String),
}

/// `!command args` as a command; None when the message is not one. Bad arguments give `Help`.
pub fn parse(text: &str) -> Option<Command> {
    let rest = text.trim().strip_prefix('!')?;
    let (word, args) = rest.split_once(char::is_whitespace).map(|(w, a)| (w, a.trim())).unwrap_or((rest, ""));
    // "!restart10" / "!reiniciar2": the minutes written without a space.
    let digits = word.trim_start_matches(|c: char| !c.is_ascii_digit());
    let (word, args) = if args.is_empty() && !digits.is_empty() && digits.len() < word.len() { (&word[..word.len() - digits.len()], digits) } else { (word, args) };
    Some(match word.to_ascii_lowercase().as_str() {
        "help" | "ayuda" => Command::Help,
        "status" | "estado" => Command::Status,
        "players" | "jugadores" => Command::Players,
        "save" | "guardar" => Command::Save,
        "restart" | "reiniciar" => match args {
            "" => Command::Restart(RESTART_DEFAULT_MIN),
            "cancel" | "cancelar" => Command::RestartCancel,
            n => match n.parse::<u32>() {
                Ok(m) if (1..=RESTART_MAX_MIN).contains(&m) => Command::Restart(m),
                _ => Command::Help,
            },
        },
        "msg" | "mensaje" => match servermsg_text(args) {
            Some(t) => Command::Msg(t),
            None => Command::Help,
        },
        _ => return None,
    })
}

/// Text that fits in `servermsg "..."`: one line, no quotes or backslashes, at most 200 characters.
pub fn servermsg_text(text: &str) -> Option<String> {
    let clean: String = text.chars().filter(|c| !matches!(c, '"' | '\\' | '\r' | '\n')).take(200).collect();
    let clean = clean.trim().to_string();
    (!clean.is_empty()).then_some(clean)
}

/// A restart asked for: when it happens, and which warnings were given.
#[derive(Debug, Clone, PartialEq)]
pub struct Countdown {
    pub server: String,
    pub at: u64,
    pub warned: BTreeSet<u32>,
}

impl Countdown {
    pub fn new(server: &str, now: u64, minutes: u32) -> Self {
        let mut warned = BTreeSet::new();
        // The warning for the time asked is the answer itself, and none is due above it.
        warned.extend([5u32, 1].into_iter().filter(|m| *m >= minutes));
        Countdown { server: server.to_string(), at: now + minutes as u64 * 60, warned }
    }

    /// The warnings due now (minutes left: 5 and 1), each once; None once it is time.
    pub fn step(&mut self, now: u64) -> Option<Vec<u32>> {
        if now >= self.at {
            return None;
        }
        let left_secs = self.at - now;
        let mut due = Vec::new();
        for m in [5u32, 1] {
            if left_secs <= m as u64 * 60 && !self.warned.contains(&m) {
                self.warned.insert(m);
                due.push(m);
            }
        }
        Some(due)
    }
}

fn say(language: &str, key: &str, args: &[&str]) -> String {
    let es = language.starts_with("es");
    let t = match (key, es) {
        ("help", false) => "Commands: `!status`, `!players`, `!restart [minutes]` (1-60, default 5), `!restart cancel`, `!save`, `!msg <text>`.",
        ("help", true) => "Comandos: `!estado`, `!jugadores`, `!reiniciar [minutos]` (1-60, 5 si no decís), `!reiniciar cancelar`, `!guardar`, `!mensaje <texto>`. También en inglés: `!status`, `!players`, `!restart`, `!save`, `!msg`.",
        ("noRole", false) => "Only members with the role **{0}** can use these commands.",
        ("noRole", true) => "Solo los miembros con el rol **{0}** pueden usar estos comandos.",
        ("down", false) => "🔴 **{0}** is not running.",
        ("down", true) => "🔴 **{0}** no está en línea.",
        ("up", false) => "🟢 **{0}** is up, {1} player(s) online.",
        ("up", true) => "🟢 **{0}** está en línea, {1} jugador(es) conectado(s).",
        ("players", false) => "Online on **{0}**: {1}",
        ("players", true) => "En línea en **{0}**: {1}",
        ("nobody", false) => "Nobody is on **{0}**.",
        ("nobody", true) => "No hay nadie en **{0}**.",
        ("restartAsked", false) => "🔄 **{0}** restarts in {1} minute(s); players are warned in the game. `!restart cancel` stops it.",
        ("restartAsked", true) => "🔄 **{0}** se reinicia en {1} minuto(s); los jugadores reciben el aviso en el juego. `!reiniciar cancelar` lo detiene.",
        ("restartCancelled", false) => "The restart of **{0}** was cancelled.",
        ("restartCancelled", true) => "Se canceló el reinicio de **{0}**.",
        ("noRestart", false) => "No restart is waiting.",
        ("noRestart", true) => "No hay ningún reinicio esperando.",
        ("restartNow", false) => "🔄 Restarting **{0}** now.",
        ("restartNow", true) => "🔄 Reiniciando **{0}** ahora.",
        ("restartStarted", false) => "🟢 **{0}** saved, stopped and is starting again (players can join once it has loaded).",
        ("restartStarted", true) => "🟢 **{0}** guardó, se detuvo y está arrancando de nuevo (se puede entrar cuando termine de cargar).",
        ("restartFailed", false) => "⚠️ **{0}** could not be started again: {1}",
        ("restartFailed", true) => "⚠️ **{0}** no se pudo volver a iniciar: {1}",
        ("saved", false) => "💾 **{0}** saved the world.",
        ("saved", true) => "💾 **{0}** guardó el mundo.",
        ("sent", false) => "Said in the game.",
        ("sent", true) => "Dicho en el juego.",
        ("noRcon", false) => "⚠️ **{0}** has no RCON password, so the Studio cannot talk to it. Stop it and start it again from the Studio: it sets one.",
        ("noRcon", true) => "⚠️ **{0}** no tiene contraseña RCON, así que el Studio no puede hablarle. Detenelo y volvé a iniciarlo desde el Studio: le pone una.",
        ("failed", false) => "⚠️ It did not work: {0}",
        ("failed", true) => "⚠️ No funcionó: {0}",
        ("gameWarn", false) => "The server restarts in {0} minute(s).",
        ("gameWarn", true) => "El servidor se reinicia en {0} minuto(s).",
        ("gameCancel", false) => "The restart was cancelled.",
        ("gameCancel", true) => "Se canceló el reinicio.",
        _ => key,
    };
    let mut out = t.to_string();
    for (i, a) in args.iter().enumerate() {
        out = out.replace(&format!("{{{}}}", i), a);
    }
    out
}

// ------------------------------------------------------------------ Discord

fn agent() -> ureq::Agent {
    let tls = ureq::tls::TlsConfig::builder()
        .provider(ureq::tls::TlsProvider::NativeTls)
        .root_certs(ureq::tls::RootCerts::PlatformVerifier)
        .build();
    ureq::Agent::config_builder()
        .tls_config(tls)
        .timeout_global(Some(Duration::from_secs(15)))
        .http_status_as_error(false)
        .user_agent(format!("DiscordBot (https://github.com/pzmodstudio, {})", env!("CARGO_PKG_VERSION")))
        .build()
        .into()
}

/// One call to Discord's API; a rate limit is waited out once.
fn call(token: &str, path: &str, body: Option<&Value>) -> Result<Value, String> {
    for _ in 0..2 {
        let url = format!("{}{}", API, path);
        let auth = format!("Bot {}", token);
        let result = match body {
            Some(b) => agent().post(&url).header("Authorization", &auth).send_json(b),
            None => agent().get(&url).header("Authorization", &auth).call(),
        };
        let mut response = result.map_err(|e| format!("unreachable: Discord could not be reached: {}", e))?;
        let status = response.status().as_u16();
        let value = response.body_mut().read_json::<Value>().unwrap_or(Value::Null);
        if (200..300).contains(&status) {
            return Ok(value);
        }
        if status == 429 {
            let wait = value.get("retry_after").and_then(|r| r.as_f64()).unwrap_or(2.0);
            std::thread::sleep(Duration::from_millis((wait.clamp(0.5, 30.0) * 1000.0) as u64));
            continue;
        }
        return Err(match status {
            401 => "bad_token: Discord does not take the bot's token (reset in the Developer Portal?).".to_string(),
            403 => "forbidden: The bot may not read or write in that channel (its permissions in Discord).".to_string(),
            _ => format!("Discord answered {} to {}.", status, path),
        });
    }
    Err("Discord asked to wait twice.".to_string())
}

/// Where the commands are read: the guild, the channel and the role, by the names given.
#[derive(Debug, Clone, PartialEq)]
struct Place {
    guild: String,
    channel: String,
    role: String,
}

fn find_place(token: &str, channel_name: &str, role_name: &str) -> Result<Place, String> {
    let channel_name = channel_name.trim().trim_start_matches('#');
    let role_name = role_name.trim().trim_start_matches('@');
    let guilds = call(token, "/users/@me/guilds", None)?;
    for g in guilds.as_array().cloned().unwrap_or_default() {
        let Some(gid) = g.get("id").and_then(|v| v.as_str()) else { continue };
        let channels = call(token, &format!("/guilds/{}/channels", gid), None)?;
        let Some(channel) = channels.as_array().into_iter().flatten().find(|c| c.get("name").and_then(|n| n.as_str()) == Some(channel_name) && c.get("type").and_then(|t| t.as_u64()) == Some(0)) else { continue };
        let roles = call(token, &format!("/guilds/{}/roles", gid), None)?;
        let role = roles.as_array().into_iter().flatten().find(|r| r.get("name").and_then(|n| n.as_str()).is_some_and(|n| n.eq_ignore_ascii_case(role_name)));
        let Some(role) = role else { return Err(format!("no_role: There is no role called \"{}\" in that Discord.", role_name)) };
        return Ok(Place {
            guild: gid.to_string(),
            channel: channel.get("id").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
            role: role.get("id").and_then(|v| v.as_str()).unwrap_or_default().to_string(),
        });
    }
    Err(format!("no_channel: The bot sees no text channel called \"{}\" (is it invited to that Discord?).", channel_name))
}

fn reply(token: &str, channel: &str, to: &str, text: &str) {
    let body = json!({
        "content": text,
        "allowed_mentions": { "parse": [] },
        "message_reference": { "message_id": to, "fail_if_not_exists": false },
    });
    if let Err(e) = call(token, &format!("/channels/{}/messages", channel), Some(&body)) {
        crate::diagnostics::error(format!("[Discord commands] {}", e));
    }
}

/// The messages after `after` (oldest first), or the newest one's id to start from.
fn read_messages(token: &str, channel: &str, after: Option<&str>) -> Result<Vec<Value>, String> {
    let path = match after {
        Some(a) => format!("/channels/{}/messages?after={}&limit=50", channel, a),
        None => format!("/channels/{}/messages?limit=1", channel),
    };
    let mut list = call(token, &path, None)?.as_array().cloned().unwrap_or_default();
    list.sort_by_key(|m| m.get("id").and_then(|v| v.as_str()).and_then(|s| s.parse::<u64>().ok()).unwrap_or(0));
    Ok(list)
}

// ------------------------------------------------------------------ doing them

fn now_secs() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn players(z: &Path) -> Vec<String> {
    crate::server_manager::get_connected_players(z.to_string_lossy().to_string()).map(|l| l.into_iter().map(|p| p.username).collect()).unwrap_or_default()
}

fn running(z: &Path, server: &str) -> bool {
    crate::server_manager::get_dedicated_server_status(z.to_string_lossy().to_string()).is_ok_and(|s| s.is_running && s.server_name.as_deref() == Some(server))
}

/// Stops the server and starts it again the way the Studio last did.
fn restart_now(z: &Path, server: &str) -> Result<(), String> {
    let z_str = z.to_string_lossy().to_string();
    crate::server_manager::stop_dedicated_server(z_str.clone(), None)?;
    // Its ports are let go a moment after the process ends.
    std::thread::sleep(Duration::from_secs(3));
    let (name, memory, nosteam) = crate::server_manager::LAST_LAUNCH.lock().ok().and_then(|l| l.clone()).unwrap_or((server.to_string(), None, None));
    let name = if name == server { name } else { server.to_string() };
    crate::server_manager::launch_dedicated_server(String::new(), z_str, name, memory, nosteam, None).map(|_| ())
}

/// What the watcher keeps between looks.
#[derive(Default)]
struct State {
    key: String,
    place: Option<Place>,
    last: Option<String>,
    roles: BTreeMap<String, (u64, Vec<String>)>,
    backoff_until: u64,
    backoff: u64,
    /// The last failure written to the log: the same one again is not written again.
    logged: Option<String>,
    countdown: Option<Countdown>,
}

static STARTED: AtomicBool = AtomicBool::new(false);

/// How the commands are doing, for the window: the server, and the channel read or what failed.
#[derive(Debug, Clone, serde::Serialize)]
pub struct CommandsStatus {
    pub server: String,
    pub listening: bool,
    pub channel: String,
    /// "code: English text", empty while it works.
    pub error: String,
}

static STATUS: std::sync::Mutex<Option<CommandsStatus>> = std::sync::Mutex::new(None);

pub fn status() -> Option<CommandsStatus> {
    STATUS.lock().ok().and_then(|s| s.clone())
}

fn set_status(value: Option<CommandsStatus>) {
    if let Ok(mut s) = STATUS.lock() {
        *s = value;
    }
}

/// Reads the commands channel of the server that has them on, while the Studio runs.
pub fn watch() {
    if STARTED.swap(true, Ordering::Relaxed) {
        return;
    }
    std::thread::spawn(|| {
        let mut st = State::default();
        loop {
            std::thread::sleep(EVERY);
            if let Err(e) = round(&mut st) {
                if let Some((server, s)) = zomboid_dir().and_then(|z| crate::discord::commands_server(&z)) {
                    set_status(Some(CommandsStatus { server, listening: false, channel: s.command_channel, error: e.clone() }));
                }
                let now = now_secs();
                st.backoff = (st.backoff * 2).clamp(10, MAX_BACKOFF_SECS);
                st.backoff_until = now + st.backoff;
                // Once per failure: a channel that is not there was written every 10 s, all day.
                if st.logged.as_deref() != Some(e.as_str()) {
                    crate::diagnostics::error(format!("[Discord commands] {} (trying again, waiting longer each time up to {} s)", e, MAX_BACKOFF_SECS));
                    st.logged = Some(e);
                }
            } else if now_secs() >= st.backoff_until {
                // A round that ran (not one skipped while waiting): the wait starts over.
                st.backoff = 0;
                st.logged = None;
            }
        }
    });
}

fn zomboid_dir() -> Option<std::path::PathBuf> {
    let chosen = crate::vfs::saved_user_zomboid_dir();
    crate::load_order::mod_info::get_existing_user_zomboid_dirs(&chosen).into_iter().next()
}

fn round(st: &mut State) -> Result<(), String> {
    let now = now_secs();
    let Some(z) = zomboid_dir() else { return Ok(()) };
    let Some((server, s)) = crate::discord::commands_server(&z) else {
        st.place = None;
        set_status(None);
        return Ok(());
    };
    let lang = s.language.clone();

    // A restart counting down: the warnings in the game, then the restart.
    if let Some(cd) = st.countdown.as_mut() {
        match cd.step(now) {
            Some(due) => {
                for m in due {
                    let _ = crate::server_manager::rcon_command(&z, &cd.server, &format!("servermsg \"{}\"", say(&lang, "gameWarn", &[&m.to_string()])));
                }
            }
            None => {
                let target = cd.server.clone();
                st.countdown = None;
                let place = st.place.clone();
                let z2 = z.clone();
                let token = crate::discord::game_bot_token(&z, &server).unwrap_or_default();
                let lang2 = lang.clone();
                std::thread::spawn(move || {
                    let post = |text: String| {
                        if let Some(p) = &place {
                            let body = json!({ "content": text, "allowed_mentions": { "parse": [] } });
                            let _ = call(&token, &format!("/channels/{}/messages", p.channel), Some(&body));
                        }
                    };
                    post(say(&lang2, "restartNow", &[&target]));
                    match restart_now(&z2, &target) {
                        Ok(()) => post(say(&lang2, "restartStarted", &[&target])),
                        Err(e) => post(say(&lang2, "restartFailed", &[&target, &e])),
                    }
                });
            }
        }
    }

    if now < st.backoff_until {
        return Ok(());
    }
    let Some(token) = crate::discord::game_bot_token(&z, &server) else {
        return Err("no_token: The server's .ini has no bot token (Chat with the game, above).".to_string());
    };
    let key = format!("{}|{}|{}|{}", server, s.command_channel, s.command_role, token.len());
    if st.key != key || st.place.is_none() {
        st.place = Some(find_place(&token, &s.command_channel, &s.command_role)?);
        st.key = key;
        st.last = None;
    }
    let place = st.place.clone().unwrap();
    set_status(Some(CommandsStatus { server: server.clone(), listening: true, channel: s.command_channel.trim().trim_start_matches('#').to_string(), error: String::new() }));
    let Some(after) = st.last.clone() else {
        // From now on: what was written before the Studio looked is not a command for it.
        st.last = Some(read_messages(&token, &place.channel, None)?.last().and_then(|m| m.get("id").and_then(|v| v.as_str()).map(str::to_string)).unwrap_or_else(|| "0".into()));
        return Ok(());
    };
    for m in read_messages(&token, &place.channel, Some(&after))? {
        let id = m.get("id").and_then(|v| v.as_str()).unwrap_or_default().to_string();
        st.last = Some(id.clone());
        if m.pointer("/author/bot").and_then(|b| b.as_bool()).unwrap_or(false) {
            continue;
        }
        let Some(cmd) = parse(m.get("content").and_then(|c| c.as_str()).unwrap_or_default()) else { continue };
        let user = m.pointer("/author/id").and_then(|v| v.as_str()).unwrap_or_default().to_string();
        let roles = match st.roles.get(&user).filter(|(at, _)| now.saturating_sub(*at) < ROLES_FOR_SECS) {
            Some((_, r)) => r.clone(),
            None => {
                let member = call(&token, &format!("/guilds/{}/members/{}", place.guild, user), None).unwrap_or(Value::Null);
                let r: Vec<String> = member.get("roles").and_then(|r| r.as_array()).into_iter().flatten().filter_map(|v| v.as_str().map(str::to_string)).collect();
                st.roles.insert(user.clone(), (now, r.clone()));
                r
            }
        };
        if !roles.contains(&place.role) {
            reply(&token, &place.channel, &id, &say(&lang, "noRole", &[&s.command_role]));
            continue;
        }
        let answer = run(&z, &server, &lang, cmd, &mut st.countdown, now);
        reply(&token, &place.channel, &id, &answer);
    }
    Ok(())
}

fn failed(lang: &str, server: &str, error: &str) -> String {
    if error.contains("no RCONPassword") {
        say(lang, "noRcon", &[server])
    } else {
        say(lang, "failed", &[error])
    }
}

/// Does one command and says how it went.
fn run(z: &Path, server: &str, lang: &str, cmd: Command, countdown: &mut Option<Countdown>, now: u64) -> String {
    let up = running(z, server);
    match cmd {
        Command::Help => say(lang, "help", &[]),
        Command::Status => {
            if up {
                say(lang, "up", &[server, &players(z).len().to_string()])
            } else {
                say(lang, "down", &[server])
            }
        }
        Command::Players => {
            if !up {
                return say(lang, "down", &[server]);
            }
            let list = players(z);
            if list.is_empty() { say(lang, "nobody", &[server]) } else { say(lang, "players", &[server, &crate::discord::names(lang, &list)]) }
        }
        Command::Save => match crate::server_manager::rcon_command(z, server, "save") {
            Ok(()) => say(lang, "saved", &[server]),
            Err(e) => failed(lang, server, &e),
        },
        Command::Msg(text) => match crate::server_manager::rcon_command(z, server, &format!("servermsg \"{}\"", text)) {
            Ok(()) => say(lang, "sent", &[]),
            Err(e) => failed(lang, server, &e),
        },
        Command::Restart(minutes) => {
            if !up {
                return say(lang, "down", &[server]);
            }
            // Without RCON the players would not be warned and the stop would not save: not done.
            if let Err(e) = crate::server_manager::rcon_command(z, server, &format!("servermsg \"{}\"", say(lang, "gameWarn", &[&minutes.to_string()]))) {
                return failed(lang, server, &e);
            }
            *countdown = Some(Countdown::new(server, now, minutes));
            say(lang, "restartAsked", &[server, &minutes.to_string()])
        }
        Command::RestartCancel => {
            if countdown.take().is_some() {
                let _ = crate::server_manager::rcon_command(z, server, &format!("servermsg \"{}\"", say(lang, "gameCancel", &[])));
                say(lang, "restartCancelled", &[server])
            } else {
                say(lang, "noRestart", &[])
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn commands_are_read_in_both_languages() {
        assert_eq!(parse("!status"), Some(Command::Status));
        assert_eq!(parse("  !Estado "), Some(Command::Status));
        assert_eq!(parse("!jugadores"), Some(Command::Players));
        assert_eq!(parse("!restart"), Some(Command::Restart(5)));
        assert_eq!(parse("!reiniciar 15"), Some(Command::Restart(15)));
        assert_eq!(parse("!restart 0"), Some(Command::Help), "1 to 60 minutes");
        assert_eq!(parse("!restart 90"), Some(Command::Help));
        assert_eq!(parse("!restart cancel"), Some(Command::RestartCancel));
        assert_eq!(parse("!reiniciar2"), Some(Command::Restart(2)), "the minutes without a space");
        assert_eq!(parse("!mp3"), None);
        assert_eq!(parse("!msg hola a \"todos\"\n"), Some(Command::Msg("hola a todos".into())), "no quotes: it would break out of servermsg");
        assert_eq!(parse("!msg"), Some(Command::Help));
        assert_eq!(parse("hello"), None);
        assert_eq!(parse("!nope"), None, "another bot's command is not ours");
    }

    #[test]
    fn a_server_without_rcon_gets_a_password_once() {
        let dir = std::env::temp_dir().join(format!("pzms_rcon_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let ini = dir.join("s.ini");
        std::fs::write(&ini, "PVP=true
RCONPort=27015
RCONPassword=
Public=false
").unwrap();
        assert!(crate::server_manager::ensure_rcon_password(&ini, &dir.join("b")).unwrap());
        let text = std::fs::read_to_string(&ini).unwrap();
        let pass = text.lines().find_map(|l| l.strip_prefix("RCONPassword=")).unwrap().to_string();
        assert!(pass.len() >= 16 && text.contains("Public=false") && text.matches("RCONPassword=").count() == 1);
        assert!(!crate::server_manager::ensure_rcon_password(&ini, &dir.join("b")).unwrap(), "kept when there is one");
        assert!(std::fs::read_to_string(&ini).unwrap().contains(&pass));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_restart_warns_at_five_and_one_minute() {
        let mut cd = Countdown::new("Coop", 1000, 10);
        assert_eq!(cd.step(1000), Some(vec![]), "the 10-minute warning was the answer itself");
        assert_eq!(cd.step(1000 + 5 * 60), Some(vec![5]));
        assert_eq!(cd.step(1000 + 5 * 60 + 10), Some(vec![]), "each once");
        assert_eq!(cd.step(1000 + 9 * 60), Some(vec![1]));
        assert_eq!(cd.step(1000 + 10 * 60), None, "time: restart");
        let mut short = Countdown::new("Coop", 0, 1);
        assert_eq!(short.step(10), Some(vec![]), "a 1-minute restart was warned when asked");
    }
}
