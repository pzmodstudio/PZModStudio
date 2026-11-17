pub mod history;
pub mod load_times;
pub mod profiler;
pub mod smooth_frames;
pub mod windows_memory;
pub mod requires;

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::Path;
use std::process::Command;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;
use tauri::Emitter;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxLaunchConfig {
    pub pz_install_dir: String,
    pub user_zomboid_dir: String,
    pub test_mode: String, // "BACKGROUND_QUICK" or "WINDOWED_DEEP"
    /// Passed to the game as they are (joining a server: `+connect ip:port`).
    #[serde(default)]
    pub extra_args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslatedErrorPayload {
    pub id: String,
    pub raw_error: String,
    pub source_file: Option<String>,
    pub line_number: Option<usize>,
    pub title: String,
    pub explanation: String,
    pub recommended_action: String,
    pub polyfill_rule_id_suggestion: Option<String>,
    /// The mod the game blamed, when the line says so.
    pub mod_name: Option<String>,
    /// Whether the game threw (ERROR, or a stack trace) or complained and carried on (WARN).
    pub severity: Severity,
    /// Which kind of card this is, so the app can say it in the player's language, with the
    /// values its texts are made of (as `HealthIssue::params`). The English texts stay as they are.
    #[serde(default)]
    pub code: String,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub params: BTreeMap<String, String>,
    /// The other mods on the stack the game printed before the error, nearest caller first: the
    /// error was in `mod_name`, called from these ("error in A, called from B").
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub called_from: Vec<String>,
    /// The mod is the player's own, in development (its folder is a link from elsewhere): theirs
    /// to mend, not something to switch off or report to an author.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub own_mod: bool,
    /// For such a mod: the file on disk that `source_file` is, when the mod's own Lua frame gave
    /// it and exactly one file of that name is among those the game loads.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub own_file: Option<String>,
    /// The mod of the Lua frame `source_file` and `line_number` came from.
    #[serde(skip)]
    pub frame_mod: Option<String>,
    /// Which log the card came from: None is the game's own console.txt; "server" the Studio's
    /// dedicated server (server-console.txt), "player2" the second game of a multiplayer test.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub log: Option<String>,
    /// The server's name, for a card from its log.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub log_name: Option<String>,
}

/// What the game itself said about the line a card came from.
///
/// The level the game wrote is the only honest measure: `WARN` means it noticed and went on
/// (`require("X") failed`, a translation with a stray `%`, even "Exception thrown" when the game
/// caught it and reported it as a warning), `ERROR` and the bare lines of a stack trace mean
/// something threw. A warning dressed as a crash made a working list look broken.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Severity {
    Exception,
    Warning,
}

impl Severity {
    pub fn of_line(line: &str) -> Severity {
        let line = line.trim_start();
        if line.starts_with("WARN") || line.starts_with("LOG") || line.starts_with("DEBUG") {
            Severity::Warning
        } else {
            Severity::Exception
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogLinePayload {
    pub line: String,
    pub is_error: bool,
}

/// Checks Windows task list to detect if ProjectZomboid64.exe is already running.
pub fn find_running_pz_pid() -> Option<u32> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        let mut cmd = Command::new("tasklist");
        cmd.args(&["/FI", "IMAGENAME eq ProjectZomboid64.exe", "/FO", "CSV", "/NH"]);
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW

        if let Ok(output) = cmd.output() {
            let text = String::from_utf8_lossy(&output.stdout);
            for line in text.lines() {
                if line.contains("ProjectZomboid64.exe") {
                    let parts: Vec<&str> = line.split(',').collect();
                    if parts.len() >= 2 {
                        let pid_str = parts[1].trim().trim_matches('"');
                        if let Ok(pid) = pid_str.parse::<u32>() {
                            return Some(pid);
                        }
                    }
                }
            }
        }
    }
    None
}

/// Configures options.ini for windowed or fullscreen mode before launching.
pub fn configure_options_ini(user_zomboid_dir: &str, is_windowed: bool) {
    for user_dir in crate::load_order::mod_info::get_all_user_zomboid_dirs(user_zomboid_dir) {
        let options_path = user_dir.join("options.ini");
        if options_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&options_path) {
                let mut new_lines = Vec::new();
                let mut found_fullscreen = false;

                // Only the one line the launch mode is about. Everything else the player set —
                // borderless above all — is theirs and is left alone.
                for line in content.lines() {
                    if line.starts_with("fullScreen=") {
                        new_lines.push(format!("fullScreen={}", !is_windowed));
                        found_fullscreen = true;
                    } else {
                        new_lines.push(line.to_string());
                    }
                }

                if !found_fullscreen {
                    new_lines.push(format!("fullScreen={}", !is_windowed));
                }

                let updated = new_lines.join("\r\n");
                if updated != content {
                    let _ = crate::load_order::ini_parser::write_file_with_backup(
                        &options_path,
                        &updated,
                        &user_dir.join("PZModStudio_Backups"),
                    );
                }
            }
        }
    }
}

/// A launch that is not the multiplayer test: the test's choices for the Studio Link
/// (`Lua/PZModStudio_MpAuto.txt`, character and spawn pressed by themselves) are taken away, or the
/// Link would build the character on its own in a normal game too. Player 1 of the test plays in
/// the player's own Zomboid folder, so the file was left there.
pub(crate) fn forget_mp_test_automation(user_zomboid_dir: &str) {
    let _ = std::fs::remove_file(Path::new(user_zomboid_dir).join("Lua").join("PZModStudio_MpAuto.txt"));
}

/// Java agents of mods (Zombie Buddy) in the game's launch file only when the mod list has the mod
/// on: they load outside the mod list, so otherwise they run with any profile.
fn sync_java_agents_for_launch(install_dir: &Path, user_zomboid_dir: &str) {
    let Some(z_dir) = crate::load_order::mod_info::get_existing_user_zomboid_dirs(user_zomboid_dir).into_iter().next() else {
        return;
    };
    // Only a list read whole: an unreadable default.txt must not take the agents out.
    if let Some(active) = crate::load_order::ini_parser::try_parse_default_txt(&z_dir.join("mods").join("default.txt")) {
        if let Err(e) = crate::studio_link::sync_java_agents_when_free(install_dir, &z_dir, &active) {
            crate::diagnostics::error(format!("[StudioLink] {}", e));
        }
    }
    // The memory the player chose, back in the launch file if a game update put its own there.
    crate::game_memory::reapply_for_launch(install_dir, &z_dir);
    // The companion mods the player has installed run this build's scripts, not an older Studio's.
    crate::claude_agent::refresh_if_installed(&z_dir);
    let z = z_dir.to_string_lossy();
    if z_dir.join("mods").join("Z_PZModStudio_Bridge").is_dir() && bridge_companion_mod_needs_update(&z) {
        if let Err(e) = install_bridge_companion_mod(&z) {
            crate::diagnostics::error(format!("[Bridge] {}", e));
        }
    }
    // The packages run this build's shim too, not the one of the Studio that published them.
    let refreshed = crate::patch_generator::refresh_installed_polyfills(&z_dir);
    if !refreshed.is_empty() {
        crate::diagnostics::log("INFO", &format!("[Shim] {} package copies brought up to this build", refreshed.len()));
    }
    // A copy an older Studio could not move into Zomboid/mods (a server held the files) left the
    // mod missing and a half-made folder the game loaded: put right, here and in the multiplayer
    // test's second folder.
    for dir in [z_dir.clone(), crate::server_manager::mp_test::second_dir(&z_dir)] {
        for done in crate::server_manager::java_mods::repair_copy_leftovers(&dir) {
            crate::diagnostics::log("INFO", &format!("[Copies] {}: {}", dir.display(), done));
        }
    }
}

/// Spawns or attaches to an active Project Zomboid process and monitors console.txt in real time.
pub fn launch_sandbox_and_watch<R: tauri::Runtime>(
    app_handle: tauri::AppHandle<R>,
    config: SandboxLaunchConfig,
) -> Result<u32, String> {
    let install_dir = Path::new(&config.pz_install_dir);
    let exe_path = install_dir.join("ProjectZomboid64.exe");

    if !exe_path.exists() {
        return Err(format!("ProjectZomboid64.exe not found at: {}", exe_path.display()));
    }

    let is_normal = config.test_mode.contains("NORMAL");
    let is_windowed = config.test_mode.contains("WINDOWED");

    // Only a launch that names a screen mode sets one. Bisect's "NORMAL" names none, and used to
    // switch a player who plays in a window to fullscreen.
    if is_windowed || config.test_mode.contains("FULLSCREEN") {
        configure_options_ini(&config.user_zomboid_dir, is_windowed);
    }
    sync_java_agents_for_launch(install_dir, &config.user_zomboid_dir);
    forget_mp_test_automation(&config.user_zomboid_dir);

    // Launch Project Zomboid directly with independent null stdio to ensure clean GUI instantiation
    let is_already_running = find_running_pz_pid().is_some();
    let pid = if let Some(existing_pid) = find_running_pz_pid() {
        existing_pid
    } else {
        let mut cmd = Command::new(&exe_path);
        cmd.current_dir(install_dir);

        if !is_normal {
            cmd.arg("-debug");
        }

        // A launch that names no screen mode keeps the player's own.
        if is_windowed {
            cmd.arg("-windowed");
        } else if config.test_mode.contains("FULLSCREEN") {
            cmd.arg("-fullscreen");
        }
        // Compatible copies in Zomboid/mods must win over their Workshop originals.
        cmd.args(crate::compat::launch_args(&config.user_zomboid_dir));
        cmd.args(&config.extra_args);

        cmd.stdin(std::process::Stdio::null());
        cmd.stdout(std::process::Stdio::null());
        cmd.stderr(std::process::Stdio::null());

        let child = cmd.spawn().map_err(|e| format!("Failed to launch ProjectZomboid64.exe: {}", e))?;
        let child_pid = child.id();
        if is_windowed {
            crate::claude_agent::start_fit_window_guard();
        }
        thread::sleep(Duration::from_millis(300));
        find_running_pz_pid().unwrap_or(child_pid)
    };

    // Emit initial log message
    let app = app_handle.clone();
    let mode_desc = match config.test_mode.as_str() {
        "DEBUG_FULLSCREEN" | "MONITORED" => "Debug Mode (Fullscreen)",
        "DEBUG_WINDOWED" | "WINDOWED" | "WINDOWED_DEEP" => "Debug Mode (Windowed)",
        "NORMAL_FULLSCREEN" => "Normal Game (Fullscreen)",
        "NORMAL_WINDOWED" | "NORMAL" | "NORMAL_DIRECT" => "Normal Game (Windowed)",
        _ => "Custom Mode",
    };

    if is_already_running {
        let _ = app.emit("sandbox-log", LogLinePayload {
            line: format!("[PZ Monitor Center] Project Zomboid is ALREADY RUNNING (PID: {}). Attached to active session! ({})", pid, mode_desc),
            is_error: false,
        });
    } else {
        let _ = app.emit("sandbox-log", LogLinePayload {
            line: format!("[PZ Monitor Center] Launched new ProjectZomboid64.exe session! PID: {} | {}", pid, mode_desc),
            is_error: false,
        });
    }

    watch_console_log(app);

    Ok(pid)
}

/// Only one thread ever tails `console.txt`, however many times the game is launched: a second
/// one would double every line and every crash card.
static LOG_WATCHER_STARTED: AtomicBool = AtomicBool::new(false);

/// Follows the game's `console.txt` and sends each new line to the Monitor Center, whether the
/// game was launched from the Studio or from Steam. When the file is not there yet — a player who
/// has never run the game has no log — it waits, and says so instead of sitting silent.
pub fn watch_console_log<R: tauri::Runtime>(app: tauri::AppHandle<R>) {
    if LOG_WATCHER_STARTED.swap(true, Ordering::Relaxed) {
        return;
    }
    thread::spawn(move || {
        let mut file_offset: Option<u64> = None;
        let mut announced_wait = false;
        let mut watching: Option<std::path::PathBuf> = None;
        let mut cards = CardMaker::new();
        // The session's error lines by shape, kept across sessions ("first time" / "seen before").
        let mut history: Option<history::Tracker> = None;

        loop {
            // The folder can be set or corrected while the Studio is open, so look again each time.
            let found = crate::load_order::mod_info::get_all_user_zomboid_dirs("")
                .into_iter()
                .map(|d| d.join("console.txt"))
                .find(|p| p.is_file());

            let Some(console_txt_path) = found else {
                if !announced_wait {
                    announced_wait = true;
                    let _ = app.emit(
                        "sandbox-log",
                        LogLinePayload {
                            line: "[PZ Monitor Center] No console.txt yet. Start Project Zomboid once and its log appears here.".to_string(),
                            is_error: false,
                        },
                    );
                }
                thread::sleep(Duration::from_millis(1000));
                continue;
            };

            if watching.as_deref() != Some(console_txt_path.as_path()) {
                let _ = app.emit(
                    "sandbox-log",
                    LogLinePayload {
                        line: format!("[PZ Monitor Center] Following {}", console_txt_path.display()),
                        is_error: false,
                    },
                );
                watching = Some(console_txt_path.clone());
                announced_wait = false;
                // Start at the end: the Studio reports what happens from now on.
                file_offset = std::fs::metadata(&console_txt_path).map(|m| m.len()).ok();
                // The history counts the session whole: what the file already holds goes in now.
                let dir = console_txt_path.parent().map(|d| d.to_path_buf()).unwrap_or_default();
                let seed: Vec<String> = std::fs::read(&console_txt_path)
                    .map(|b| String::from_utf8_lossy(&b).lines().map(|l| l.trim_end().to_string()).collect())
                    .unwrap_or_default();
                let mut tracker = history::Tracker::load(&dir);
                tracker.start_session(&seed);
                history = Some(tracker);
            }

            if let Ok(file) = File::open(&console_txt_path) {
                let mut offset = file_offset.unwrap_or(0);
                if let Ok(metadata) = file.metadata() {
                    if metadata.len() < offset {
                        // A new session truncated the file: read it from the top, and forget the
                        // last session -- what it blamed, and which lines were already reported.
                        // Without this a relaunch reported nothing it had reported before, so a
                        // mod's warning that survived a fix never came back as a card, and the
                        // Monitor said two warnings over a log that held six.
                        offset = 0;
                        cards.new_session();
                        if let Some(h) = history.as_mut() {
                            h.start_session(&[]);
                        }
                        let _ = app.emit("sandbox-session-started", ());
                    }
                }
                let mut reader = BufReader::new(file);
                if reader.seek(SeekFrom::Start(offset)).is_ok() {
                    let mut line_buf: Vec<u8> = Vec::new();
                    let mut read_any = false;
                    while reader.read_until(b'\n', &mut line_buf).unwrap_or(0) > 0 {
                        read_any = true;
                        let trimmed = String::from_utf8_lossy(&line_buf).trim_end().to_string();
                        let is_error = trimmed.contains("ERROR")
                            || trimmed.contains("Exception")
                            || trimmed.contains("KahluaThreadException");

                        let _ = app.emit("sandbox-log", LogLinePayload { line: trimmed.clone(), is_error });
                        if let Some(h) = history.as_mut() {
                            h.see(&trimmed);
                        }
                        for mut card in cards.line(&trimmed) {
                            mark_own_mod_installed(&mut card);
                            let _ = app.emit("sandbox-error-card", card);
                        }
                        line_buf.clear();
                    }
                    // Nothing more came: an error's card waiting for its exception line goes out.
                    if !read_any {
                        if let Some(mut card) = cards.flush() {
                            mark_own_mod_installed(&mut card);
                            let _ = app.emit("sandbox-error-card", card);
                        }
                    }
                    file_offset = reader.stream_position().ok();
                }
            }
            if let Some(h) = history.as_mut() {
                h.flush_if_due();
            }
            thread::sleep(Duration::from_millis(250));
        }
    });
}

static OTHER_LOGS_WATCHER_STARTED: AtomicBool = AtomicBool::new(false);

/// One log besides the game's that cards are made from, as it is followed.
struct OtherLog {
    /// "server" or "player2": what `TranslatedErrorPayload::log` says.
    kind: &'static str,
    offset: Option<u64>,
    cards: CardMaker,
}

/// The cards of the other logs the Studio runs: its dedicated server's `server-console.txt` and,
/// in a multiplayer test, player 2's `console.txt`. The same card maker as the game's, each card
/// marked with its log (and the server's name), so the Monitor shows a log's own cards beside it
/// and a fix says which list it reaches. The lines themselves are read by the Monitor's tabs; only
/// cards come from here. A log rewritten from the top (a new server start) starts its cards over
/// and says so with `sandbox-log-session-started` (its kind), so only that log's cards are cleared.
pub fn watch_other_logs<R: tauri::Runtime>(app: tauri::AppHandle<R>) {
    if OTHER_LOGS_WATCHER_STARTED.swap(true, Ordering::Relaxed) {
        return;
    }
    thread::spawn(move || {
        let mut logs = [
            OtherLog { kind: "server", offset: None, cards: CardMaker::new() },
            OtherLog { kind: "player2", offset: None, cards: CardMaker::new() },
        ];
        loop {
            // The Zomboid folder the Studio is set to (Settings), else the one it finds.
            let saved = crate::vfs::studio_paths().user_zomboid_dir;
            let Some(primary) = crate::load_order::mod_info::get_existing_user_zomboid_dirs(&saved).into_iter().next() else {
                thread::sleep(Duration::from_millis(2000));
                continue;
            };
            for log in logs.iter_mut() {
                let path = match log.kind {
                    "server" => primary.join("server-console.txt"),
                    _ => crate::server_manager::mp_test::second_dir(&primary).join("console.txt"),
                };
                let kind = log.kind;
                let name = || match kind {
                    "server" => crate::server_manager::get_dedicated_server_status(primary.to_string_lossy().into_owned()).ok().and_then(|s| s.server_name),
                    _ => None,
                };
                let (cards, restarted) = follow_other_log(log, &path);
                if restarted {
                    let _ = app.emit("sandbox-log-session-started", kind);
                }
                for mut card in cards {
                    // Each log numbers its own cards: the kind keeps them apart from the game's.
                    card.id = format!("{}-{}", kind, card.id);
                    card.log = Some(kind.to_string());
                    card.log_name = name();
                    mark_own_mod_installed(&mut card);
                    let _ = app.emit("sandbox-error-card", card);
                }
            }
            thread::sleep(Duration::from_millis(500));
        }
    });
}

/// What one other log added since the last look, as cards, and whether the log started over (a
/// new server start rewrote it). Starts at the end of the file the first time (what is new from
/// now on, as the game's watcher does).
fn follow_other_log(log: &mut OtherLog, path: &std::path::Path) -> (Vec<TranslatedErrorPayload>, bool) {
    let Ok(file) = File::open(path) else { return (Vec::new(), false) };
    let len = file.metadata().map(|m| m.len()).unwrap_or(0);
    let Some(mut offset) = log.offset else {
        log.offset = Some(len);
        return (Vec::new(), false);
    };
    let restarted = len < offset;
    if restarted {
        offset = 0;
        log.cards.new_session();
    }
    let mut out = Vec::new();
    let mut reader = BufReader::new(file);
    if reader.seek(SeekFrom::Start(offset)).is_err() {
        return (out, restarted);
    }
    let mut buf: Vec<u8> = Vec::new();
    let mut read_any = false;
    let mut at = offset;
    while reader.read_until(b'\n', &mut buf).unwrap_or(0) > 0 {
        // A line still being written (no newline yet) is read again whole next time.
        if !buf.ends_with(b"\n") {
            break;
        }
        read_any = true;
        at += buf.len() as u64;
        out.extend(log.cards.line(String::from_utf8_lossy(&buf).trim_end()));
        buf.clear();
    }
    if !read_any {
        out.extend(log.cards.flush());
    }
    log.offset = Some(at);
    (out, restarted)
}

/// A line the game wrote at a level of its own (`ERROR: ...`, `LOG  : ...`); the lines in between
/// (a Java exception, a stack) belong to the level line above them.
fn is_level_line(line: &str) -> bool {
    let t = line.trim_start();
    ["LOG", "ERROR", "WARN", "DEBUG", "TRACE"].iter().any(|l| t.starts_with(l)) && !line.starts_with(char::is_whitespace)
}

/// Turns console lines into Monitor cards, one per error.
///
/// The game writes an error as an ERROR line and, under it, the Java exception that says what
/// went wrong (`java.lang.RuntimeException: attempted index: ... of non-table: null`) and its
/// stack. Both name the mod, and each used to become a card: two cards for one error. Now the
/// ERROR's card waits until its block ends and takes the exception line as what it says, or
/// gives way to the card of a pattern the Studio knows in that line.
///
/// The Java stack under the ERROR lists the Lua levels too, each with its mod
/// (`Lua((MOD:X)).fn(File.lua:48)`, `Lua(Vanilla).fn(...)`), the one that failed first: the mods
/// below it called it, and the card says so ("called from"). The `STACK TRACE` dump the game
/// prints elsewhere comes after the error, introduced by a `flushErrorMessage` line, and next to
/// another error it is easy to take for that one's: only the block's own stack counts.
pub struct CardMaker {
    blame: ModBlame,
    /// One card per mod and complaint: a mod that fails on every tick would otherwise bury
    /// everything else.
    already_reported: std::collections::HashSet<String>,
    counter: usize,
    /// The card of the ERROR whose block is being read.
    pending: Option<TranslatedErrorPayload>,
    /// Whether that card already took an exception line.
    pending_has_detail: bool,
    /// The lines being read are under an ERROR (reported or not): its exception and stack.
    in_error: bool,
    /// The mods of the Lua levels on the held card's stack, in order.
    block_mods: Vec<String>,
    /// The texts the game could not format, one group per mod (or none) and kind.
    texts: Vec<TextGroup>,
}

/// How many keys a card of texts spells out; the rest are counted.
const TEXT_KEYS_SHOWN: usize = 8;

/// The texts of one mod (or of none the Studio found) that the game could not format, one kind:
/// a bare `%`, or a placeholder given no value. One card for all of them, sent again with the same
/// id each time a new key joins, so whoever shows the cards replaces it instead of adding one.
struct TextGroup {
    owner: Option<String>,
    missing: bool,
    keys: Vec<String>,
    card: TranslatedErrorPayload,
}

impl TextGroup {
    /// The card's words for the keys it holds now. Every sentence is what the lines say: which
    /// texts, that the game showed them as they are and went on, and whose files hold them when
    /// the Studio found them there.
    fn rewrite(&mut self) {
        let n = self.keys.len();
        let many = n > 1;
        let shown = self.keys.iter().take(TEXT_KEYS_SHOWN).cloned().collect::<Vec<_>>().join(", ");
        let more = n.saturating_sub(TEXT_KEYS_SHOWN);
        let more_text = if more > 0 { format!(" and {} more", more) } else { String::new() };
        let whose = match (&self.owner, many) {
            (Some(m), false) => format!("It is in {}'s translation files.", m),
            (Some(m), true) => format!("They are in {}'s translation files.", m),
            (None, false) => "The Studio did not find it in the texts of the mods you have on; it can be the game's own.".to_string(),
            (None, true) => "The Studio did not find them in the texts of the mods you have on; they can be the game's own.".to_string(),
        };
        let card = &mut self.card;
        match (self.missing, many) {
            (false, false) => {
                card.title = "A text has a % sign the game cannot format".to_string();
                card.explanation = format!("A translated text has a % that is not written %%, so Java's text formatting cannot read it. The game shows the text as it is and carries on. The text: {}. {}", shown, whose);
                card.recommended_action = "Nothing to do to play. When a mod ships the text, the fix is in its translation file (% written as %%); Save report names the mod, the file and the line.".to_string();
            }
            (false, true) => {
                card.title = format!("{} texts have a % sign the game cannot format", n);
                card.explanation = format!("{} translated texts have a % that is not written %%, so Java's text formatting cannot read them. The game shows each text as it is and carries on. The texts: {}{}. {}", n, shown, more_text, whose);
                card.recommended_action = "Nothing to do to play. When a mod ships the texts, the fix is in its translation files (% written as %%); Save report names the mod, the file and the line of the first one.".to_string();
            }
            (true, false) => {
                card.title = "A text has a placeholder and was given no value".to_string();
                card.explanation = format!("A translated text has a placeholder (%1, %s) and the code that asked for it passed no value. The game shows the text with the placeholder still in it and carries on. The text: {}. {}", shown, whose);
                card.recommended_action = "Nothing to do to play. Save report names the key and, when a mod ships it, the mod and the file.".to_string();
            }
            (true, true) => {
                card.title = format!("{} texts have a placeholder and were given no value", n);
                card.explanation = format!("{} translated texts have a placeholder (%1, %s) and the code that asked for them passed no value. The game shows each text with the placeholder still in it and carries on. The texts: {}{}. {}", n, shown, more_text, whose);
                card.recommended_action = "Nothing to do to play. Save report names the first key and, when a mod ships it, the mod and the file.".to_string();
            }
        }
        let variant = match (self.missing, many) {
            (false, false) => "",
            (false, true) => "many",
            (true, false) => "missing",
            (true, true) => "missing_many",
        };
        let whose_key = match (self.owner.is_some(), many) {
            (true, false) => "text_whose_mod_one",
            (true, true) => "text_whose_mod_many",
            (false, false) => "text_whose_none_one",
            (false, true) => "text_whose_none_many",
        };
        card.params = crate::catalog::issue_params(&[
            ("variant", variant),
            ("count", &n.to_string()),
            ("keys", &shown),
            // Empty, not "0", when all are shown: the app shows the value as it is when no phrase stands in for it.
            ("more", &if more > 0 { more.to_string() } else { String::new() }),
            ("moreKey", if more > 0 { "and_more" } else { "" }),
            ("mod", self.owner.as_deref().unwrap_or("")),
            ("whoseKey", whose_key),
        ]);
    }
}

impl CardMaker {
    pub fn new() -> Self {
        CardMaker { blame: ModBlame::default(), already_reported: Default::default(), counter: 1, pending: None, pending_has_detail: false, in_error: false, block_mods: Vec::new(), texts: Vec::new() }
    }

    /// A new game session (console.txt started over): what it blamed and reported is forgotten.
    pub fn new_session(&mut self) {
        self.blame = ModBlame::default();
        self.already_reported.clear();
        self.pending = None;
        self.in_error = false;
        self.texts.clear();
    }

    /// A text the game could not format joins the card of its mod and kind; the card comes back
    /// (the same id as before) when the key is new to it.
    fn text_card(&mut self, mut card: TranslatedErrorPayload, line: &str) -> Option<TranslatedErrorPayload> {
        let key = crate::error_report::formatting_key(line).unwrap_or_else(|| "(a text the line does not name)".to_string());
        let missing = card.params.get("variant").is_some_and(|v| v == "missing");
        let at = match self.texts.iter().position(|g| g.owner == card.mod_name && g.missing == missing) {
            Some(at) => at,
            None => {
                card.id = format!("err_{}", self.counter);
                self.counter += 1;
                self.texts.push(TextGroup { owner: card.mod_name.clone(), missing, keys: Vec::new(), card });
                self.texts.len() - 1
            }
        };
        let group = &mut self.texts[at];
        if group.keys.contains(&key) {
            return None;
        }
        group.keys.push(key);
        group.rewrite();
        Some(group.card.clone())
    }

    /// The card that was waiting for the rest of its block.
    pub fn flush(&mut self) -> Option<TranslatedErrorPayload> {
        let mut card = self.pending.take()?;
        // The mods below the one that failed, each once, nearest first (at most three).
        let culprit = card.mod_name.clone().unwrap_or_default();
        let mut callers: Vec<String> = Vec::new();
        for name in self.block_mods.drain(..) {
            if name != culprit && !callers.contains(&name) {
                callers.push(name);
            }
        }
        callers.truncate(3);
        card.called_from = callers;
        lua_failure_in_words(&mut card);
        Some(card)
    }

    fn hold(&mut self, card: TranslatedErrorPayload) {
        self.pending = Some(card);
        self.pending_has_detail = false;
        self.block_mods.clear();
    }

    /// The cards this line completes (usually none, at most two).
    pub fn line(&mut self, line: &str) -> Vec<TranslatedErrorPayload> {
        let line = without_log_time(line);
        let mut out = Vec::new();
        self.blame.observe(line);
        let level = is_level_line(line);
        if level {
            out.extend(self.flush());
            self.in_error = false;
        }
        let trimmed = line.trim();
        if !level && self.pending.is_some() {
            if let Some(name) = ModBlame::named_in(trimmed) {
                self.block_mods.push(name);
            }
            // The first Lua frame under the ERROR is where it failed: its file and line go on the card.
            if let (Some(held), Some((file, number))) = (self.pending.as_mut(), lua_frame_place(trimmed)) {
                if held.line_number.is_none() {
                    held.source_file = Some(file);
                    held.line_number = Some(number);
                    held.frame_mod = ModBlame::named_in(trimmed);
                }
            }
        }
        let is_error_level = level && trimmed.starts_with("ERROR");
        let culprit = self.blame.of(line);
        if self.blame.java_echo() {
            return out;
        }
        if let Some(mut card) = translate_log_error(line, self.counter).filter(|c| level && c.code == "translator_text") {
            // One card for a mod's texts of one kind, whichever of them the line is about.
            card.mod_name = culprit;
            out.extend(self.text_card(card, line));
        } else if let Some(mut card) = translate_log_error(line, self.counter) {
            self.counter += 1;
            if names_java_mod_by_tag(&card.code) {
                // The tag a Java mod signs its lines with -> the name in the mod list.
                let tag = card.params.get("mod").cloned().unwrap_or_default();
                // `[ZB] [!] class viewpoint.Main: error invoking main()`: ZombieBuddy (which signs
                // as ZB) could not run a Java mod's start-up code. The mod is the one whose jar the
                // class's package is; ZombieBuddy goes on loading its patches.
                let zb_main = (tag == "ZB")
                    .then(|| line.split("class ").nth(1).filter(|_| line.contains("error invoking main()")).and_then(|r| r.split('.').next()).map(str::to_string))
                    .flatten();
                if let Some(package) = zb_main {
                    let name = self.blame.jar_mod(&package).unwrap_or(package.clone());
                    card.mod_name = Some(name.clone());
                    card.title = format!("ZombieBuddy could not run {}'s start-up code", name);
                    card.explanation = format!(
                        "ZombieBuddy started {}'s Java (its main) and that code failed; ZombieBuddy went on and loaded its patches. On a dedicated server this is expected for a mod that draws the picture (a server draws nothing). In the game it means what {} sets up when it starts may be missing.",
                        name, name
                    );
                    card.recommended_action = format!(
                        "Nothing to do on a dedicated server. In the game, if {} does not work, update it; its Workshop page often says which version of the game it needs.",
                        name
                    );
                    card.params = crate::catalog::issue_params(&[("mod", &name), ("variant", "zb_main")]);
                } else if tag == "ZB" {
                    card.title = card.title.replacen("ZB", "ZombieBuddy", 1);
                    card.params.insert("mod".to_string(), "ZombieBuddy".to_string());
                    card.mod_name = Some("ZombieBuddy".to_string());
                } else if let Some(name) = self.blame.jar_mod(&tag) {
                    card.params.insert("mod".to_string(), name.clone());
                    card.title = card.title.replacen(&tag, &name, 1);
                    card.mod_name = Some(name);
                } else if card.mod_name.is_none() && !tag.is_empty() {
                    card.mod_name = Some(tag);
                }
            } else {
                card.mod_name = culprit.clone();
            }
            if card.code == "java_patch_failed" {
                let suspects = self.blame.transform_suspects(line);
                let said = if suspects.is_empty() { "none found in the jars loaded".to_string() } else { suspects.join(", ") };
                card.explanation = card.explanation.replacen("(see the log)", &said, 1);
                card.params.insert("suspects".to_string(), said);
                card.params.insert("suspectsKey".to_string(), if suspects.is_empty() { "none_in_jars".to_string() } else { String::new() });
            }
            if card.code == "java_shader_failed" {
                // The code that uses an undefined name is often an addon's, put into the host's
                // shader: when exactly one other loaded jar carries shader code with that name,
                // the card says so.
                let name = card.params.get("name").cloned().unwrap_or_default();
                let host = card.mod_name.clone().unwrap_or_default();
                let suspects = self.blame.shader_suspects(&name, &host);
                if let [addon] = suspects.as_slice() {
                    card.params.insert("addon".to_string(), addon.clone());
                    card.params.insert("variant".to_string(), "addon".to_string());
                    card.explanation = format!(
                        "{} could not build a shader: it uses \"{}\", which was not defined when it compiled. Shader code that uses \"{}\" is in {}'s jar (an addon that adds to {}'s shaders), so that is the likely source, perhaps made for another version of {}. What that shader draws is missing.",
                        host, name, name, addon, host, host
                    );
                    card.recommended_action = format!(
                        "Nothing breaks the game; that part of the picture (often the lighting) is missing or wrong. If it bothers you, switch {} off or update it, and see whether this card goes away; {} works without it.",
                        addon, host
                    );
                }
            }
            // A Java mod's own failures come once per shader, per pass on a class, per stage: one
            // card each for what failed and whose.
            if matches!(card.code.as_str(), "java_patch_failed" | "java_mod_turned_off" | "java_shader_failed" | "java_addon_incompatible") {
                let key = format!("{}|{}|{}", card.code, card.mod_name.as_deref().unwrap_or(""), card.title);
                if !self.already_reported.insert(key) {
                    return out;
                }
            }
            let generic = matches!(card.code.as_str(), "nil_table" | "java_npe" | "kahlua");
            let held_names_mod = self.pending.as_ref().is_some_and(|h| h.code.starts_with("mod_named") || h.code == "package_copy");
            if !level && generic && self.in_error && self.pending.is_none() {
                // Under an ERROR whose card was already shown (the same failure again, every tick):
                // its exception is that same error, not a new one. An ERROR in the game's own file
                // (`Lua(Vanilla).getMissingMods> Exception thrown`) names no mod and makes no card:
                // its exception is the only card it gets, once per wording.
                let key = format!("unnamed|{}", without_frame_stamp(trimmed).chars().take(80).collect::<String>());
                if culprit.is_none() && self.already_reported.insert(key) {
                    out.push(card);
                }
            } else if !level && generic && held_names_mod {
                // The exception under an ERROR that already named its mod: the card keeps the mod
                // and takes the game's own words, which say more than a general card would.
                if let Some(held) = self.pending.as_mut() {
                    if !self.pending_has_detail {
                        held.explanation = trimmed.to_string();
                        self.pending_has_detail = true;
                    }
                }
            } else if !level && self.pending.is_some() {
                // A pattern the Studio knows, in the exception under an ERROR: that card for it.
                let held = self.pending.take().unwrap();
                card.mod_name = held.mod_name.or(card.mod_name);
                self.pending = Some(card);
                self.pending_has_detail = true;
            } else if is_error_level {
                self.hold(card);
            } else {
                out.push(card);
            }
        } else if let Some(name) = culprit {
            if !level && self.in_error {
                // The exception line under the ERROR: what its card says.
                let exception = trimmed.contains("Exception") || trimmed.contains("Error");
                if let Some(held) = self.pending.as_mut() {
                    if exception && !self.pending_has_detail && (held.code.starts_with("mod_named") || held.code == "package_copy") {
                        held.explanation = trimmed.to_string();
                        self.pending_has_detail = true;
                    }
                }
            } else {
                // Not a pattern the Studio knows, but the game named the mod, which is the
                // question a player actually has: which one of my mods did this?
                // `ERROR ... at KahluaThread.flushErrorMessage > ----` introduces the game's dump
                // of the stack of an error already reported: not an error of its own.
                let complaining = (line.contains("ERROR") || line.contains("Exception") || line.contains("KahluaThreadException") || trimmed.starts_with("WARN")) && !trimmed.contains("flushErrorMessage");
                // The same complaint whatever frame it came on: in play the game stamps every line
                // with its frame ("f:156", "st:910,647,017"), and a mod failing on every frame
                // made a card per frame.
                let key = format!("{}|{}", name, without_frame_stamp(trimmed).chars().take(80).collect::<String>());
                if complaining && self.already_reported.insert(key) {
                    let card = blame_card(trimmed, &name, self.counter);
                    self.counter += 1;
                    if is_error_level {
                        self.hold(card);
                    } else {
                        out.push(card);
                    }
                }
            }
        }
        if is_error_level {
            self.in_error = true;
        }
        out
    }
}

impl Default for CardMaker {
    fn default() -> Self {
        Self::new()
    }
}

/// Which mod a log line belongs to.
///
/// The game is explicit about this in two ways, so no guessing is needed:
///   `WARN : Lua f:0 at Lua((MOD:Nature's Call)).NC_Main.lua> require(...) failed`
///   `LOG  : Mod f:0> mod "CleanUI" overrides media/lua/client/isui/isinventorypane.lua`
/// The first names the mod on the line itself; the second builds a map from a file path to
/// whichever mod replaced it, which covers lines that only mention a path.
#[derive(Debug, Clone, Default)]
pub struct ModBlame {
    /// media path (lowercased) -> the mod that overrides it, last one winning as the game loads.
    overrides: std::collections::HashMap<String, String>,
    /// The last `Lua((MOD:X))` seen and how many lines ago, because the game prints the failing
    /// stack frame first and the ERROR line after it:
    ///
    /// ```text
    ///     Lua((MOD:Fancy Handwork B42.20)).FHhandleKeybinds(aFancyHandwork.lua:126)
    /// ERROR: General f:240> 20)).FHhandleKeybinds> Exception thrown
    /// ```
    ///
    /// The ERROR line names nobody -- the game even truncates the mod's name to "20))" -- so
    /// read on its own it is one more anonymous Java exception, which is how 413 of them in one
    /// session went unattributed.
    recent: Option<(String, usize)>,
    /// A Java mod's own error line (`[BloodFX] drawing disabled: java.lang.X: ...`) and the
    /// exception it names: the mod's printStackTrace repeats that exception on the next line.
    java_last: Option<String>,
    /// This line is that repeat: the same error, already carded under the mod's name.
    java_echo: bool,
    /// `recent` came from a "cannot create item ... modID" line, not from a stack frame.
    recent_is_item: bool,
    /// Java mods ZombieBuddy put on the classpath (`[ZB] added to classpath: <jar>`): the jar's
    /// name without `.jar`, lowercased -> the mod's name from its `mod.info`. A Java mod's own
    /// lines carry a tag, often its jar's name (`[ViewpointHeadlightPlacement]`), which is not
    /// the name the player sees in the mod list ("Viewpoint - Light Fix").
    jars: std::collections::HashMap<String, String>,
    /// The mod of the jar ZombieBuddy put on the classpath last: its patches are applied next.
    loading_jar: Option<String>,
    /// Every jar put on the classpath: (mod name, jar path, package ZombieBuddy scanned in it).
    loaded_jars: Vec<(String, String, Option<String>)>,
    /// The `@Patch` targets (class, method) read from each of those jars, when first needed.
    jar_targets: std::collections::HashMap<String, Vec<(String, String, Vec<String>)>>,
    /// An undefined shader name -> the loaded addons whose jar carries shader code using it.
    shader_users: std::collections::HashMap<String, Vec<String>>,
    /// The methods ZombieBuddy is patching in the pass under way (`[ZB] patching <class>.<method>`).
    pass_methods: Vec<(String, String)>,
    /// `[ZB] [!] ERROR transforming <class>: <why>` -> the mod it is said of (None when the
    /// jars leave more than one) and the mods whose patches were in that pass. The same failure
    /// comes back each time a later mod patches that class (the patches of a class are applied
    /// together) and keeps what was found the first time.
    transform_first: std::collections::HashMap<String, (Option<String>, Vec<String>)>,
}

/// `[ZB] [!] ERROR transforming zombie.characters.IsoZombie: Cannot assign float arg2 to double.`
/// -> `zombie.characters.IsoZombie: Cannot assign float arg2 to double`.
fn zb_transform_failure(line: &str) -> Option<&str> {
    let rest = line.split("[ZB] [!] ERROR transforming ").nth(1)?;
    let rest = rest.trim().trim_end_matches('.');
    rest.contains(": ").then_some(rest)
}

/// The mod a jar belongs to, from the `mod.info` of a folder above it
/// (`...\mods\ViewpointLight\42\media\java\client\X.jar` -> `...\ViewpointLight\42\mod.info`);
/// the mod's folder name when there is none to read.
fn mod_of_jar(jar: &str) -> Option<String> {
    let path = Path::new(jar);
    // `mods/X/42/media/java/client/X.jar`: the mod.info is in `42/` or, for a mod that keeps it
    // there, in `common/` beside it (Viewpoint RTX).
    let infos = path.ancestors().skip(1).take(6).flat_map(|dir| {
        let common = (dir.file_name().is_some_and(|n| n != "common")).then(|| dir.parent().map(|p| p.join("common").join("mod.info"))).flatten();
        std::iter::once(dir.join("mod.info")).chain(common)
    });
    for info in infos {
        if let Ok(text) = std::fs::read_to_string(&info) {
            let name = text.lines().find_map(|l| l.trim().strip_prefix("name=")).map(str::trim).filter(|n| !n.is_empty());
            if let Some(name) = name {
                return Some(name.to_string());
            }
        }
    }
    let normal = jar.replace('/', "\\");
    let (_, after) = normal.rsplit_once("\\mods\\")?;
    after.split('\\').next().filter(|f| !f.is_empty()).map(str::to_string)
}

/// How many lines a mod's name carries forward. The frame and its error are next to each other;
/// a wider window would start blaming whoever happened to be mentioned earlier.
const BLAME_WINDOW: usize = 3;

impl ModBlame {
    /// The line just observed repeats the exception of a Java mod's own error line above it.
    pub fn java_echo(&self) -> bool {
        self.java_echo
    }

    /// Reads `mod "X" overrides <path>` lines as they go by, and remembers a mod named in a
    /// stack frame for the few lines that follow it.
    pub fn observe(&mut self, line: &str) {
        self.java_echo = self.java_last.take().is_some_and(|thrown| {
            let text = line.split_once("> ").map(|(_, rest)| rest).unwrap_or(line).trim();
            text.starts_with("java.") && thrown.starts_with(text.get(..text.len().min(120)).unwrap_or(text))
        });
        if java_mod_own_error(line).is_some() {
            self.java_last = line.split_once(": java.").map(|(_, rest)| format!("java.{}", rest.trim()));
        }
        if let Some(jar) = line.split("[ZB] added to classpath: ").nth(1) {
            let jar = jar.trim().trim_end_matches('.');
            let stem = Path::new(jar).file_stem().map(|s| s.to_string_lossy().to_lowercase()).unwrap_or_default();
            self.loading_jar = mod_of_jar(jar);
            if let (false, Some(name)) = (stem.is_empty(), self.loading_jar.clone()) {
                self.jars.insert(stem, name.clone());
                self.loaded_jars.push((name, jar.to_string(), None));
            }
        }
        // `[ZB] Scanned 32 classes in package vpheadlights.`: the package of the jar just loaded.
        if let Some(rest) = line.split("[ZB] Scanned ").nth(1) {
            if let (Some(package), Some(last)) = (rest.split(" in package ").nth(1), self.loaded_jars.last_mut()) {
                last.2.get_or_insert_with(|| package.trim().trim_end_matches('.').to_string());
            }
        }
        if let Some(rest) = line.split("[ZB] patching ").nth(1) {
            if let Some((full, _)) = rest.split_once(" with ") {
                if let Some((class, method)) = full.rsplit_once('.') {
                    self.pass_methods.push((class.to_string(), method.to_string()));
                }
            }
        } else if line.contains("[ZB] Transformed: ") {
            self.pass_methods.clear();
        }
        if let Some(failure) = zb_transform_failure(line) {
            if !self.transform_first.contains_key(failure) {
                let (class, why) = failure.split_once(": ").unwrap_or((failure, ""));
                let found = self.patch_suspects(class, why);
                self.transform_first.insert(failure.to_string(), found);
            }
            self.pass_methods.clear();
        }
        self.recent = match self.recent.take() {
            Some((name, age)) if age + 1 < BLAME_WINDOW => Some((name, age + 1)),
            _ => None,
        };
        if let Some(name) = Self::named_in(line) {
            // The Studio's own wrappers (the shim's event guards, the profiler) sit in the stack
            // under the mod's handler: they do not take the blame from the mod beside them.
            let keep = is_studio_mod_name(&name) && self.recent.as_ref().is_some_and(|(n, _)| !is_studio_mod_name(n));
            if !keep {
                self.recent = Some((name, 0));
                self.recent_is_item = false;
            }
        } else if let Some(id) = Self::item_mod(line) {
            // `CreateItem() cannot create item: ... modID = "X"` comes right before the exception
            // the game throws loading that item from a save.
            self.recent = Some((id, 0));
            self.recent_is_item = true;
        } else if Self::is_vanilla_frame(line) {
            // The game's own file is the nearer frame: what follows is not the mod's.
            self.recent = None;
        }
        let Some(rest) = line.split("> mod \"").nth(1) else { return };
        let Some((mod_name, tail)) = rest.split_once("\" overrides ") else { return };
        self.overrides.insert(tail.trim().to_lowercase(), mod_name.to_string());
    }

    /// The mod a line names itself, in `Lua((MOD:X))`.
    fn named_in(line: &str) -> Option<String> {
        let rest = line.split("(MOD:").nth(1)?;
        let (name, _) = rest.split_once(')')?;
        let name = name.trim();
        (!name.is_empty()).then(|| name.to_string())
    }

    /// The mod of an item the game could not create: `... cannot create item: ... modID = "X"`.
    fn item_mod(line: &str) -> Option<String> {
        if !line.contains("cannot create item") {
            return None;
        }
        let rest = line.split("modID = \"").nth(1)?;
        let id = rest.split('"').next()?.trim();
        (!id.is_empty()).then(|| id.to_string())
    }

    /// `Lua(Vanilla)`: the game says the file is its own, not a mod's.
    fn is_vanilla_frame(line: &str) -> bool {
        line.contains("Lua(Vanilla)")
    }

    /// The mod a Java mod's log tag stands for: `ViewpointHeadlightPlacement` -> "Viewpoint -
    /// Light Fix", when a jar of that name was put on the classpath this session.
    /// Else the one jar whose name ends with the tag: `[TrueBallistics]` signs for
    /// `ViewpointTrueBallistics.jar` ("Viewpoint True Ballistics" in the mod list).
    pub fn jar_mod(&self, tag: &str) -> Option<String> {
        let tag = tag.trim().to_lowercase();
        if let Some(name) = self.jars.get(&tag) {
            return Some(name.clone());
        }
        if tag.len() < 6 {
            return None;
        }
        let mut ends: Vec<&String> = self.jars.iter().filter(|(jar, _)| jar.ends_with(&tag)).map(|(_, name)| name).collect();
        ends.sort();
        ends.dedup();
        (ends.len() == 1).then(|| ends[0].clone())
    }

    /// Whose patch failed in a pass on `class`: the loaded jars whose `@Patch` names one of the
    /// pass's methods. The jar being loaded when it is one of them (its patches are the ones being
    /// added: a class already loaded is patched again as each jar comes); otherwise the only one;
    /// otherwise nobody, with all of them listed. The class a lazily loaded class's pass names is
    /// unrelated to whatever jar was loaded last (Viewpoint's camera, patched by two mods, was
    /// blamed on the jar loaded just before it).
    ///
    /// ByteBuddy's `Cannot assign float arg3 to class [F` names the type a patch declares
    /// (`class [F`, a float[]) where the method has another: of the suspects, the one whose patch
    /// to that method has a parameter of that type is the one, when it is the only one (Viewpoint
    /// Rolling Hills' camera patch, beside OldPoint's, after Viewpoint changed `place`).
    fn patch_suspects(&mut self, class: &str, why: &str) -> (Option<String>, Vec<String>) {
        let declared = why.strip_prefix("Cannot assign ").and_then(|r| r.rsplit_once(" to ")).map(|(_, t)| t.trim().to_string());
        let mut typed: Vec<String> = Vec::new();
        let methods: Vec<String> = self.pass_methods.iter().filter(|(c, _)| c == class).map(|(_, m)| m.clone()).collect();
        let mut suspects: Vec<String> = Vec::new();
        for (name, jar, package) in self.loaded_jars.clone() {
            let Some(package) = package else { continue };
            let targets = self.jar_targets.entry(jar.clone()).or_insert_with(|| {
                std::fs::read(&jar)
                    .ok()
                    .and_then(|bytes| crate::java_patches::read_jar(&bytes, &package).ok())
                    .map(|read| read.patches.into_iter().filter_map(|p| Some((p.target_class?, p.target_method?, p.advice_param_types))).collect())
                    .unwrap_or_default()
            });
            let hits: Vec<&Vec<String>> =
                targets.iter().filter(|(c, m, _)| c == class && (methods.is_empty() || methods.iter().any(|x| x == m))).map(|(_, _, t)| t).collect();
            if hits.is_empty() {
                continue;
            }
            let has_type = declared.as_ref().is_some_and(|d| hits.iter().any(|types| types.contains(d)));
            if has_type && !typed.contains(&name) {
                typed.push(name.clone());
            }
            if !suspects.contains(&name) {
                suspects.push(name);
            }
        }
        let named = match &self.loading_jar {
            _ if typed.len() == 1 => typed.first().cloned(),
            Some(loading) if suspects.contains(loading) => Some(loading.clone()),
            _ if suspects.len() == 1 => suspects.first().cloned(),
            _ => None,
        };
        (named, suspects)
    }

    /// The loaded Java mods other than `host` whose jars carry shader code that uses `name`.
    pub fn shader_suspects(&mut self, name: &str, host: &str) -> Vec<String> {
        if name.is_empty() {
            return Vec::new();
        }
        if let Some(found) = self.shader_users.get(name) {
            return found.iter().filter(|m| m.as_str() != host).cloned().collect();
        }
        let mut found: Vec<String> = Vec::new();
        for (mod_name, jar, _) in &self.loaded_jars {
            if mod_name == host || found.contains(mod_name) {
                continue;
            }
            if std::fs::read(jar).is_ok_and(|bytes| crate::java_patches::jar_shader_uses(&bytes, name)) {
                found.push(mod_name.clone());
            }
        }
        self.shader_users.insert(name.to_string(), found.clone());
        found
    }

    /// The mods whose patches were in the pass a ZombieBuddy transform failure is about.
    pub fn transform_suspects(&self, line: &str) -> Vec<String> {
        zb_transform_failure(line).and_then(|f| self.transform_first.get(f)).map(|(_, s)| s.clone()).unwrap_or_default()
    }

    /// The mod this line is about, if the game said.
    pub fn of(&self, line: &str) -> Option<String> {
        if let Some(failure) = zb_transform_failure(line) {
            return self.transform_first.get(failure).and_then(|(named, _)| named.clone());
        }
        // A line in one of the game's own files is the game's, whatever mod was named above it
        // (42.21's own TakeSameType.lua requires a module that no longer exists, for one).
        if Self::is_vanilla_frame(line) && !line.contains("(MOD:") {
            return None;
        }
        if let Some(rest) = line.split("(MOD:").nth(1) {
            if let Some((name, _)) = rest.split_once(')') {
                let name = name.trim();
                if !name.is_empty() {
                    if is_studio_mod_name(name) {
                        if let Some((other, _)) = self.recent.as_ref().filter(|(n, _)| !is_studio_mod_name(n)) {
                            return Some(other.clone());
                        }
                    }
                    return Some(name.to_string());
                }
            }
        }
        // A text the game could not format names only its key: the mod is whoever ships that
        // key, never the mod named a line or two above.
        if line.contains("Translator.reportMissingArgumentsFromPastAbuse") {
            return crate::error_report::formatting_key(line).and_then(|key| crate::error_report::translation_owner(&key));
        }
        // Otherwise: a media path in the line, if some mod replaced that file.
        let lower = line.to_lowercase();
        if let Some(start) = lower.find("media/").or_else(|| lower.find("media\\")) {
            let path: String = lower[start..]
                .chars()
                .take_while(|c| !c.is_whitespace() && *c != '"' && *c != ')' && *c != '\'')
                .collect();
            let path = path.replace('\\', "/");
            if let Some(owner) = self.overrides.get(path.trim_end_matches(|c| c == ',' || c == '.')) {
                return Some(owner.clone());
            }
        }
        // Failing both: the mod named a line or two above, but only where the line belongs to
        // that mod's block. A line of its own (an ERROR or WARN naming a Java class after "at",
        // which is how the game's own messages look) says nothing of the mod that happened to be
        // named before it: "Recipe Piano missing UiConfigScript" was blamed on whichever mod
        // loaded two lines earlier.
        if !Self::continues_block(line, self.recent_is_item) {
            return None;
        }
        self.recent.as_ref().map(|(name, _)| name.clone())
    }

    /// Whether a line that names nobody belongs to the block of the mod named just above it:
    /// the truncated frame the game prints under a Lua frame (`> 20)).FHhandleKeybinds> Exception
    /// thrown`), an indented line of a stack, the bare exception line under it, or, after a
    /// "cannot create item" line, the item load that fails next.
    fn continues_block(line: &str, after_item: bool) -> bool {
        if after_item {
            return line.contains("InventoryItem.loadItem") || line.contains("InventoryItem.load(");
        }
        let indented = line.starts_with(' ') || line.starts_with('\t');
        let text = line.trim_start();
        let level = ["ERROR", "WARN", "LOG", "DEBUG"].iter().any(|l| text.starts_with(l));
        if !level {
            return indented || text.starts_with("java.") || text.starts_with("se.krka.") || text.starts_with("Caused by") || text.starts_with("at ");
        }
        // A level line: only the Lua frame's own echo, never a line with its own source.
        line.contains(")).") && line.contains("Exception thrown")
    }

    /// Whether the line itself names a mod (`(MOD:X)`), as opposed to one named above it.
    pub fn names_on_line(line: &str) -> bool {
        line.contains("(MOD:")
    }
}

/// A mod the Studio installs (its packages, the bridge, the profiler): their names all start so.
fn is_studio_mod_name(name: &str) -> bool {
    name.starts_with("PZ Mod Studio")
}

/// The display name every Studio package gets in `generate_master_patch`: `name=PZ Mod Studio Patch: <x>`.
pub const STUDIO_PACKAGE_PREFIX: &str = "PZ Mod Studio Patch: ";

/// What one mod did to the last session's log: how many times it threw, how many times the game
/// warned about it, and the last line of each.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModErrorStat {
    /// The name the game wrote, which is the mod's display name.
    pub mod_name: String,
    pub exceptions: usize,
    pub warnings: usize,
    pub last_exception: Option<String>,
    pub last_warning: Option<String>,
    /// Every console line its trouble took: the ERROR / WARN line and the stack lines under it.
    #[serde(default)]
    pub lines: usize,
}

/// Every mod the last session's log named on a line of trouble, with its counts.
///
/// The same attribution as the Monitor's cards: the name on the line, the mod that overrides the
/// path, or the mod in the frame a few lines above. Only lines the game marked as trouble count;
/// a mod that merely loaded is not here. Studio packages are counted under their own name, which
/// is what the game writes.
pub fn error_stats_per_mod(lines: impl Iterator<Item = String>) -> Vec<ModErrorStat> {
    let mut blame = ModBlame::default();
    let mut stats: std::collections::BTreeMap<String, ModErrorStat> = Default::default();
    // The mod whose ERROR / WARN the lines below belong to (its stack), until the next level line.
    let mut block: Option<String> = None;
    for line in lines {
        blame.observe(&line);
        let trimmed = line.trim();
        let level_line = ["LOG", "ERROR", "WARN", "DEBUG", "TRACE"].iter().any(|l| trimmed.starts_with(l));
        if !level_line {
            // A frame of another mod is the start of its own error (the game prints the frame
            // first), not more of this one.
            if ModBlame::named_in(trimmed).is_some_and(|n| Some(&n) != block.as_ref()) {
                block = None;
            }
            if let Some(entry) = block.as_ref().and_then(|name| stats.get_mut(name)) {
                entry.lines += 1;
            }
            continue;
        }
        block = None;
        // Only the game's own level lines count: the Java frames under an ERROR would count the
        // same throw three or four times over.
        let is_trouble = trimmed.starts_with("ERROR") || trimmed.starts_with("WARN");
        if !is_trouble {
            continue;
        }
        let Some(name) = blame.of(trimmed) else { continue };
        block = Some(name.clone());
        let entry = stats.entry(name.clone()).or_insert_with(|| ModErrorStat { mod_name: name, ..Default::default() });
        entry.lines += 1;
        match Severity::of_line(trimmed) {
            Severity::Exception => {
                entry.exceptions += 1;
                entry.last_exception = Some(trimmed.chars().take(200).collect());
            }
            Severity::Warning => {
                entry.warnings += 1;
                entry.last_warning = Some(trimmed.chars().take(200).collect());
            }
        }
    }
    let mut out: Vec<ModErrorStat> = stats.into_values().collect();
    out.sort_by(|a, b| (b.exceptions, b.warnings).cmp(&(a.exceptions, a.warnings)));
    out
}

/// The per-mod counts of the session in `console.txt` now.
/// The lines of a log, each read as bytes and decoded leniently. A mod that prints a name in its
/// own code page leaves a line that is not UTF-8; `BufRead::lines` stops at it with an error, and
/// everything after it -- most of a session, sometimes -- was never read.
pub fn lossy_lines<R: BufRead>(mut reader: R) -> impl Iterator<Item = String> {
    let mut buf = Vec::new();
    std::iter::from_fn(move || {
        buf.clear();
        match reader.read_until(b'\n', &mut buf) {
            Ok(0) | Err(_) => None,
            Ok(_) => {
                while matches!(buf.last(), Some(b'\n') | Some(b'\r')) {
                    buf.pop();
                }
                Some(String::from_utf8_lossy(&buf).into_owned())
            }
        }
    })
}

/// The last `n` lines of a file, read from its end: a long session's console.txt runs to tens of
/// megabytes, and asking for its last hundred lines no longer reads all of it.
pub fn tail_lines(path: &Path, n: usize) -> std::io::Result<Vec<String>> {
    let mut file = File::open(path)?;
    let len = file.metadata()?.len();
    if n == 0 {
        return Ok(Vec::new());
    }
    let mut window: u64 = 64 * 1024;
    loop {
        let start = len.saturating_sub(window);
        file.seek(SeekFrom::Start(start))?;
        let mut bytes = Vec::new();
        file.by_ref().take(len - start).read_to_end(&mut bytes)?;
        let newlines = bytes.iter().filter(|b| **b == b'\n').count();
        // One line more than asked for, so the first (maybe cut) line can be dropped.
        if start == 0 || newlines > n {
            let mut slice = &bytes[..];
            if start > 0 {
                let cut = slice.iter().position(|b| *b == b'\n').map(|i| i + 1).unwrap_or(0);
                slice = &slice[cut..];
            }
            let mut lines: Vec<String> = lossy_lines(slice).collect();
            if lines.len() > n {
                lines.drain(..lines.len() - n);
            }
            return Ok(lines);
        }
        window = window.saturating_mul(4);
    }
}

/// The last count, kept while console.txt is unchanged: Diagnostics asks on every refresh, and a
/// long session's log runs to tens of megabytes.
static STATS_CACHE: std::sync::Mutex<Option<(std::path::PathBuf, u64, Option<std::time::SystemTime>, Vec<ModErrorStat>)>> = std::sync::Mutex::new(None);

pub fn error_stats_from_console(user_zomboid_dir: &str) -> Vec<ModErrorStat> {
    let Some(path) = crate::load_order::mod_info::get_all_user_zomboid_dirs(user_zomboid_dir)
        .into_iter()
        .map(|d| d.join("console.txt"))
        .find(|p| p.is_file())
    else {
        return Vec::new();
    };
    let meta = std::fs::metadata(&path).ok();
    let (len, modified) = (meta.as_ref().map(|m| m.len()).unwrap_or(0), meta.and_then(|m| m.modified().ok()));
    if let Ok(cache) = STATS_CACHE.lock() {
        if let Some((p, l, m, stats)) = cache.as_ref() {
            if *p == path && *l == len && *m == modified {
                return stats.clone();
            }
        }
    }
    let Ok(file) = File::open(&path) else { return Vec::new() };
    let stats = error_stats_per_mod(lossy_lines(BufReader::new(file)));
    if let Ok(mut cache) = STATS_CACHE.lock() {
        *cache = Some((path, len, modified, stats.clone()));
    }
    stats
}

/// A mod the player is making: its folder in Zomboid/mods is a link to a folder elsewhere.
#[derive(Debug, Clone)]
pub struct OwnMod {
    /// The name the game writes in the log (the mod's display name).
    pub name: String,
    pub id: String,
    /// The folder the link leads to.
    pub root: std::path::PathBuf,
}

/// The mods in development among the installed ones (`CatalogMod::in_development`).
pub fn own_mods_of(catalog: &crate::catalog::ModCatalog) -> Vec<OwnMod> {
    catalog
        .mods
        .iter()
        .filter(|m| m.in_development && m.shadowed_by.is_none())
        .map(|m| OwnMod { name: m.name.clone(), id: m.id.clone(), root: std::fs::canonicalize(&m.root_dir).unwrap_or_else(|_| std::path::PathBuf::from(&m.root_dir)) })
        .collect()
}

/// The file called `file_name` among the Lua the game loads from this mod, when there is exactly
/// one: the same relative path in `common/` and in the version folder is one file (the version
/// folder's wins, as in the game); two different paths with that name are two, and then none is
/// given rather than the wrong one.
pub fn own_file_of(root: &Path, game_version: Option<u32>, file_name: &str) -> Option<std::path::PathBuf> {
    let wanted = file_name.replace('\\', "/").rsplit('/').next()?.to_lowercase();
    if wanted.is_empty() {
        return None;
    }
    // Relative path (lowercased) -> the file, a later layer replacing an earlier one.
    let mut found: BTreeMap<String, std::path::PathBuf> = BTreeMap::new();
    for layer in crate::load_order::mod_info::media_layers_for_build(root, game_version) {
        let lua = layer.join("media").join("lua");
        for entry in walkdir::WalkDir::new(&lua).into_iter().filter_map(|e| e.ok()) {
            if entry.file_type().is_file() && entry.file_name().to_string_lossy().to_lowercase() == wanted {
                let rel = entry.path().strip_prefix(&lua).unwrap_or(entry.path()).to_string_lossy().replace('\\', "/").to_lowercase();
                found.insert(rel, entry.path().to_path_buf());
            }
        }
    }
    (found.len() == 1).then(|| found.into_values().next()).flatten()
}

/// Says a card is about the player's own mod, and where its file is when the card has the place
/// from that mod's own Lua frame.
pub fn mark_own_mod(card: &mut TranslatedErrorPayload, own: &[OwnMod], game_version: Option<u32>) {
    let Some(name) = card.mod_name.as_deref() else { return };
    let Some(m) = own.iter().find(|m| m.name == name || m.id == name) else { return };
    card.own_mod = true;
    let from_its_frame = card.frame_mod.as_deref().is_some_and(|f| f == m.name || f == m.id);
    if let (true, Some(file), Some(_)) = (from_its_frame, card.source_file.as_deref(), card.line_number) {
        card.own_file = own_file_of(&m.root, game_version, file).map(|p| p.to_string_lossy().trim_start_matches(r"\\?\").to_string());
    }
}

/// The player's own mods as the watcher knows them, looked at again after a minute: a card is
/// rare, and a link made while the Studio is open should show without a restart.
static OWN_MODS: std::sync::Mutex<Option<(std::time::Instant, Vec<OwnMod>, Option<u32>)>> = std::sync::Mutex::new(None);

/// `mark_own_mod` with the installed mods of the Studio's saved folders.
pub fn mark_own_mod_installed(card: &mut TranslatedErrorPayload) {
    if card.mod_name.is_none() {
        return;
    }
    // Tests see no installed mods, except the ones run against this PC (PZMS_ZOMBOID_DIR).
    if cfg!(test) && std::env::var("PZMS_ZOMBOID_DIR").is_err() {
        return;
    }
    let Ok(mut cache) = OWN_MODS.lock() else { return };
    if cache.as_ref().map(|(at, _, _)| at.elapsed() > Duration::from_secs(60)).unwrap_or(true) {
        let paths = crate::vfs::studio_paths();
        let catalog = crate::catalog::build_catalog(&paths.workshop_dir, &paths.user_zomboid_dir, None);
        *cache = Some((std::time::Instant::now(), own_mods_of(&catalog), catalog.game_version_int()));
    }
    if let Some((_, own, game)) = cache.as_ref() {
        mark_own_mod(card, own, *game);
    }
}

/// A line worth showing even though it matches no known pattern, because the game named a mod.
fn blame_card(line: &str, mod_name: &str, counter: usize) -> TranslatedErrorPayload {
    let severity = Severity::of_line(line);
    // Named on this very line, or only on a line just above it (the frame the game printed first).
    let on_line = ModBlame::names_on_line(line);
    // "WARN : Lua f:0 at Lua((MOD:X)).File.lua> require("Y") failed" -> the part after ">"
    let detail = line.split_once('>').map(|(_, rest)| rest.trim()).unwrap_or(line).to_string();
    let source_file = line
        .split("))." )
        .nth(1)
        .and_then(|rest| rest.split('>').next())
        .and_then(|f| {
            let f = f.trim();
            if f.ends_with(".lua") {
                return Some(f.to_string());
            }
            // `HandleZombiesAmputations(ZombiesAmputation.lua:86)`: the file in the parentheses.
            let inner = f.split('(').nth(1)?.split(')').next()?;
            let file = inner.split(':').next()?.trim();
            file.ends_with(".lua").then(|| file.to_string())
        });
    // The game names a Studio package when it read the package's copy of a file -- a mod's own
    // file, patched. Blaming "PZ Mod Studio Patch: MergedPatch" is true and useless: the line
    // is the mod's, and the copy either changed it or did not.
    if let Some(package) = mod_name.strip_prefix(STUDIO_PACKAGE_PREFIX) {
        let file = source_file.clone().unwrap_or_else(|| "a file".to_string());
        return TranslatedErrorPayload {
            called_from: Vec::new(),
            id: format!("err_{}", counter),
            code: "package_copy".to_string(),
            params: crate::catalog::issue_params(&[
                ("file", source_file.as_deref().unwrap_or("")),
                ("fileKey", if source_file.is_none() { "a_file" } else { "" }),
                ("package", package),
            ]),
            raw_error: line.to_string(),
            source_file,
            line_number: None,
            title: format!("{}, the patched copy in {}, reported a problem", file, package),
            explanation: if detail.is_empty() { line.to_string() } else { detail },
            recommended_action: format!(
                "The game read {}'s copy of this file, not the mod's. If the patch did not touch this line, the mod's own copy says the same thing, and a copy that changes nothing only freezes the file at today's version: unmerge it in the Merger. If it did, this is the line to look at there.",
                package
            ),
            polyfill_rule_id_suggestion: None,
            mod_name: Some(mod_name.to_string()),
            severity,
            own_mod: false,
            own_file: None,
            frame_mod: None,
            log: None,
            log_name: None,
        };
    }
    // `require("Vehicles/Supra_functions") failed`: the file asked for is in no mod that is on,
    // nor in the game (often a compatibility file for a mod the player does not have).
    let required = (severity == Severity::Warning && on_line)
        .then(|| line.split("require(\"").nth(1).and_then(|r| r.split_once("\") failed")).map(|(f, _)| f.to_string()))
        .flatten();
    // `recursive require(): .../media/lua/server/LooseEnds/Delivery.lua`: the mod's files ask for
    // each other in a loop; the game loads the file once and skips the repeat.
    let looped = (severity == Severity::Warning && on_line)
        .then(|| detail.strip_prefix("recursive require():").map(|f| {
            let f = f.trim().trim_end_matches('.').replace('\\', "/");
            f.split("/media/lua/").nth(1).map(|s| s.to_string()).unwrap_or_else(|| f.rsplit('/').next().unwrap_or(&f).to_string())
        }))
        .flatten();
    if let Some(file) = &looped {
        return TranslatedErrorPayload {
            called_from: Vec::new(),
            id: format!("err_{}", counter),
            code: "mod_named_warning".to_string(),
            params: crate::catalog::issue_params(&[("mod", mod_name), ("file", file), ("variant", "recursive_require")]),
            raw_error: line.to_string(),
            source_file,
            line_number: None,
            title: format!("{}'s files ask for each other in a loop", mod_name),
            explanation: format!(
                "{} asked for {} (require) while that file was still loading: two of its files ask for each other. The game loads the file once and skips the repeat, so usually nothing is missing.",
                mod_name, file
            ),
            recommended_action: format!(
                "Nothing to do unless something of {} is missing in game; then tell its author (Save report writes the note).",
                mod_name
            ),
            polyfill_rule_id_suggestion: None,
            mod_name: Some(mod_name.to_string()),
            severity,
            own_mod: false,
            own_file: None,
            frame_mod: None,
            log: None,
            log_name: None,
        };
    }
    if let Some(file) = &required {
        return TranslatedErrorPayload {
            called_from: Vec::new(),
            id: format!("err_{}", counter),
            code: "mod_named_warning".to_string(),
            params: crate::catalog::issue_params(&[("mod", mod_name), ("file", file), ("variant", "require")]),
            raw_error: line.to_string(),
            source_file,
            line_number: None,
            title: format!("{} asked for a file that is not there", mod_name),
            explanation: format!(
                "{} asked for the file {} (require), and the game found it neither in its own files nor in any mod that is on. Often it is a part meant for another mod you do not have (a compatibility file), and the rest of {} works.",
                mod_name, file, mod_name
            ),
            recommended_action: format!(
                "Nothing to do unless something of {} is missing in game; then its Workshop page may name a mod it needs without listing it as required.",
                mod_name
            ),
            polyfill_rule_id_suggestion: None,
            mod_name: Some(mod_name.to_string()),
            severity,
            own_mod: false,
            own_file: None,
            frame_mod: None,
            log: None,
            log_name: None,
        };
    }
    TranslatedErrorPayload {
        called_from: Vec::new(),
        id: format!("err_{}", counter),
        code: match severity {
            Severity::Exception => "mod_named_exception",
            Severity::Warning => "mod_named_warning",
        }
        .to_string(),
        params: crate::catalog::issue_params(&[("mod", mod_name), ("variant", if on_line { "" } else { "above" })]),
        raw_error: line.to_string(),
        source_file,
        line_number: None,
        title: if on_line { format!("The game named {} on this line", mod_name) } else { format!("{} was named just above this line", mod_name) },
        explanation: if detail.is_empty() { line.to_string() } else { detail },
        recommended_action: match (severity, on_line) {
            (Severity::Exception, true) => format!(
                "The game named {} on this line. If something is misbehaving in game, switching that mod off is the quickest way to tell whether it is the cause.",
                mod_name
            ),
            (Severity::Warning, true) => format!(
                "The game named {} here and carried on. Nothing to do unless something in that mod misbehaves in game; then this line is where to look.",
                mod_name
            ),
            (_, false) => format!(
                "This line names no mod: {} is in the stack frame the game printed just above it, so the error came up while that mod's code was running. Whether the mod is the cause, the lines around it in the log say.",
                mod_name
            ),
        },
        polyfill_rule_id_suggestion: None,
        mod_name: Some(mod_name.to_string()),
        severity,
        own_mod: false,
        own_file: None,
        frame_mod: None,
        log: None,
        log_name: None,
    }
}

/// What a Kahlua failure message says, in a player's words: `(kind, name)` where `name` is the
/// field or function the message names (`attempted index: getName of non-table: null` ->
/// ("index", "getName")). None for a message the Studio does not know.
fn lua_failure_kind(message: &str) -> Option<(&'static str, String)> {
    let m = message.trim().trim_start_matches("java.lang.RuntimeException:").trim();
    if m.starts_with("__concat not defined for operands") {
        return Some(("concat", String::new()));
    }
    if let Some(rest) = m.strip_prefix("attempted index: ") {
        let (name, tail) = rest.split_once(' ')?;
        return tail.starts_with("of non-table").then(|| ("index", name.to_string()));
    }
    if let Some(rest) = m.strip_prefix("Object tried to call nil in ") {
        let name = rest.split(" at ").next().unwrap_or(rest).trim().trim_end_matches('.').to_string();
        return Some(("call", name));
    }
    if ["__add", "__sub", "__mul", "__div", "__mod", "__pow", "__unm"].iter().any(|op| m.starts_with(&format!("{} not defined for operands", op))) {
        return Some(("arith", String::new()));
    }
    if ["__lt", "__le"].iter().any(|op| m.starts_with(&format!("{} not defined for operands", op))) {
        return Some(("compare", String::new()));
    }
    // `No implementation found for function: get(class zombie...PropertyContainer {...}, class ...)`:
    // a Java method of the game called with arguments it no longer takes.
    if let Some(rest) = m.strip_prefix("No implementation found for function: ") {
        let name = rest.split('(').next().unwrap_or(rest).trim().to_string();
        // The first thing it was given: `(class zombie.inventory.types.ComboItem Base.Bandage...` -> ComboItem.
        let given = rest
            .split_once('(')
            .and_then(|(_, args)| args.strip_prefix("class "))
            .and_then(|a| a.split([' ', ',', ')']).next())
            .and_then(|c| c.rsplit('.').next())
            .unwrap_or("")
            .to_string();
        return Some(("no_impl", format!("{}|{}", name, given)));
    }
    None
}

/// A mod-named error whose exception is a Lua failure the Studio knows (text joined with nil, a
/// field of nil, a function that does not exist, math or a comparison with nil): its card says
/// what happened in plain words, where (file and line when the stack gave them), that what the
/// mod was doing stopped while the game went on, and keeps the game's own words at the end.
fn lua_failure_in_words(card: &mut TranslatedErrorPayload) {
    if card.code != "mod_named_exception" || card.params.get("variant").is_some_and(|v| !v.is_empty()) {
        return;
    }
    let Some((kind, name)) = lua_failure_kind(&card.explanation) else { return };
    // `no_impl` carries the function and what it was given: `getWeaponType|ComboItem`.
    let (name, given) = match name.split_once('|') {
        Some((n, g)) => (n.to_string(), g.to_string()),
        None => (name, String::new()),
    };
    let Some(mod_name) = card.mod_name.clone() else { return };
    let place = match (&card.source_file, card.line_number) {
        (Some(f), Some(n)) => format!("{}:{}", f, n),
        (Some(f), None) => f.clone(),
        _ => String::new(),
    };
    let (short, long) = match kind {
        "concat" => ("joined text with a value that is not there".to_string(), "tried to join text with a value that does not exist (nil)".to_string()),
        "index" => (format!("used {} of something that is not there", name), format!("asked for {} of something that does not exist (nil)", name)),
        "call" => (format!("called {}, which does not exist", name), format!("called {}, a function that does not exist: renamed or removed by a game update, or part of a mod that is not loaded", name)),
        "arith" => ("did math with a value that is not there".to_string(), "did math with a value that does not exist (nil)".to_string()),
        "no_impl" => (
            format!("called the game's {} with something it does not take", name),
            format!(
                "called the game's {} with {}, which that function does not take: either the mod passed the wrong kind of thing, or the function changed in a game update",
                name,
                if given.is_empty() { "a value".to_string() } else { format!("a {}", given) }
            ),
        ),
        _ => ("compared a value that is not there".to_string(), "compared a value that does not exist (nil)".to_string()),
    };
    let game_words = card.explanation.trim().to_string();
    card.title = format!("{}: {}", mod_name, short);
    card.explanation = format!(
        "{} {}{}. What it was doing at that moment stopped; the game carried on. The game's words: {}",
        mod_name,
        long,
        if place.is_empty() { String::new() } else { format!(", in {}", place) },
        game_words
    );
    card.recommended_action = format!(
        "If something of {} does not work in game, this is the likely reason. It is a mistake in the mod's own code: tell its author (Save report writes the note, with the file and the line). Switching {} off tells whether it is the cause.",
        mod_name, mod_name
    );
    card.params.insert("variant".to_string(), "lua".to_string());
    card.params.insert("luaKey".to_string(), format!("lua_{}", kind));
    card.params.insert("luaLongKey".to_string(), format!("lua_{}_long", kind));
    card.params.insert("name".to_string(), name);
    card.params.insert("given".to_string(), given.clone());
    card.params.insert("givenKey".to_string(), if given.is_empty() { "lua_given_value".to_string() } else { "lua_given".to_string() });
    card.params.insert("place".to_string(), place.clone());
    card.params.insert("placeKey".to_string(), if place.is_empty() { String::new() } else { "lua_place".to_string() });
    card.params.insert("words".to_string(), game_words);
}

/// A line of a session log the game kept in `Logs/` (`[16-11-26 16:47:46.533] LOG  : ...`) as it
/// stood in `console.txt` (`LOG  : ...`).
fn without_log_time(line: &str) -> &str {
    if let Some(rest) = line.strip_prefix('[') {
        if let Some((stamp, after)) = rest.split_once("] ") {
            if stamp.len() <= 24 && stamp.chars().all(|c| c.is_ascii_digit() || matches!(c, '-' | ':' | '.' | ' ')) {
                return after;
            }
        }
    }
    line
}

/// The UI method a line says was indexed on nil (`attempted index: prerender of non-table:
/// null`): the lifecycle methods a window inherits from its base class.
fn ui_method_of_nil(line: &str) -> Option<&'static str> {
    const METHODS: [&str; 8] = ["prerender", "render", "update", "initialise", "createChildren", "onMouseMove", "onMouseDown", "onMouseUp"];
    let rest = line.split("attempted index: ").nth(1)?;
    let (method, tail) = rest.split_once(' ')?;
    if !tail.starts_with("of non-table: null") {
        return None;
    }
    METHODS.iter().copied().find(|m| *m == method)
}

/// `[Tag] rest` written by a Java mod: its tag and what follows.
fn java_mod_tag_line(line: &str) -> Option<(String, String)> {
    let text = line.split_once("> ").map(|(_, rest)| rest).unwrap_or(line).trim_start();
    let (tag, rest) = text.strip_prefix('[')?.split_once(']')?;
    let ok_tag = !tag.is_empty() && tag.len() <= 40 && tag.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == ' ');
    ok_tag.then(|| (tag.trim().to_string(), rest.trim().trim_end_matches('.').trim_end_matches(':').trim().to_string()))
}

/// A Java mod saying it stopped working for the rest of the session: `[Viewpoint] crashed while
/// preparing: ...`, `[Viewpoint] disabled after error while preparing:`, `[ViewpointHeadlightPlacement]
/// Disabled after API error: java.lang.ExceptionInInitializerError`.
///
/// Also a line at its own ERROR level saying the mod as a whole is disabled: `[TrueBallistics]
/// ERROR True Ballistics disabled: a required game method is missing (game update?). Vanilla
/// firearm combat is used.` (42.22.2). A WARN about one of its features (`... feature disabled`)
/// is not the mod switching off.
fn java_mod_turned_off(line: &str) -> Option<(String, String)> {
    let (tag, rest) = java_mod_tag_line(line)?;
    let lower = rest.to_lowercase();
    let after_error = lower.strip_prefix("[error]").or_else(|| lower.strip_prefix("error")).map(str::trim_start);
    let whole_mod_off = after_error.is_some_and(|t| t.contains(" disabled") && !t.contains("feature"));
    let off = lower.starts_with("crashed while") || (lower.starts_with("disabled after") && (lower.contains("error") || lower.contains("exception") || lower.contains("crash"))) || whole_mod_off;
    let rest = if whole_mod_off { rest[rest.len() - after_error.unwrap_or_default().len()..].to_string() } else { rest };
    off.then_some((tag, rest))
}

/// A Java mod's shader that did not compile: `[Viewpoint] light, default shader compile failed
/// (light.frag): lib/spots.glsl(116) : error C1503: undefined variable "headlightGain"`. The tag
/// and the name the shader used and nothing defined, when the line gives one.
fn java_mod_shader_failed(line: &str) -> Option<(String, Option<String>)> {
    let (tag, rest) = java_mod_tag_line(line)?;
    if !rest.contains("shader compile failed") {
        return None;
    }
    let undefined = rest.split("undefined variable \"").nth(1).and_then(|v| v.split('"').next()).filter(|v| !v.is_empty()).map(str::to_string);
    Some((tag, undefined))
}

/// A Java mod naming an addon of its own that does not fit it: `[Viewpoint] model program:
/// [ViewpointDoorLighting] Incompatible model shader; missing: ...` -> (`Viewpoint`,
/// `ViewpointDoorLighting`, `Incompatible model shader; missing: ...`).
fn java_addon_incompatible(line: &str) -> Option<(String, String, String)> {
    let (host, rest) = java_mod_tag_line(line)?;
    let (_, after) = rest.split_once("program: [")?;
    let (addon, what) = after.split_once(']')?;
    let what = what.trim();
    let ok = !addon.is_empty() && addon.len() <= 40 && addon.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-');
    (ok && what.to_lowercase().starts_with("incompatible")).then(|| (host, addon.to_string(), what.trim_end_matches(';').to_string()))
}

/// Codes of cards about a Java mod that name it by its log tag, which `CardMaker` turns into the
/// mod's name when a jar of that name was loaded.
fn names_java_mod_by_tag(code: &str) -> bool {
    matches!(code, "java_mod_error" | "java_mod_turned_off" | "java_shader_failed" | "java_addon_incompatible")
}

/// Translates raw Java stacktraces & Lua errors into actionable cards.
pub fn translate_log_error(line: &str, counter: usize) -> Option<TranslatedErrorPayload> {
    let mut card = translate_log_error_rule(line, counter)?;
    // A rule that knows the game carried on says so; a rule that reports an exception is still only
    // a warning when the game itself wrote the line at WARN/LOG level -- except a mod's Java patch
    // not applied and a Java mod switching itself off: written at LOG level by ZombieBuddy or the
    // mod, and something of the mod no longer works.
    if card.severity == Severity::Exception && !matches!(card.code.as_str(), "java_patch_failed" | "java_mod_turned_off" | "java_addon_incompatible") {
        card.severity = Severity::of_line(line);
    }
    Some(card)
}

/// `[Tag] what: java.x.SomeException ...` written by a Java mod (the game's own lines never start
/// with a bracketed tag): the tag and what the mod says happened.
fn java_mod_own_error(line: &str) -> Option<(String, String)> {
    let text = line.split_once("> ").map(|(_, rest)| rest).unwrap_or(line).trim_start();
    let rest = text.strip_prefix('[')?;
    let (tag, rest) = rest.split_once(']')?;
    let (what, thrown) = rest.split_once(": java.")?;
    // The game ends each line it logs with a full stop: `java.lang.ExceptionInInitializerError.`
    let class = thrown.split(|c: char| c == ':' || c.is_whitespace()).next().unwrap_or("").trim_end_matches('.');
    let ok_tag = !tag.is_empty() && tag.len() <= 40 && tag.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-' || c == ' ');
    (ok_tag && (class.ends_with("Exception") || class.ends_with("Error"))).then(|| (tag.trim().to_string(), what.trim().to_string()))
}

/// A log line without the frame stamps the game puts on it in play: `f:156` and `st:910,647,017`.
fn without_frame_stamp(line: &str) -> String {
    let mut out = String::with_capacity(line.len());
    let mut words = line.split(' ').peekable();
    while let Some(word) = words.next() {
        let stamp = |prefix: &str| word.strip_prefix(prefix).is_some_and(|rest| {
            let rest = rest.trim_end_matches('>');
            !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit() || c == ',')
        });
        if stamp("f:") || stamp("st:") {
            if word.ends_with('>') {
                out.push('>');
                out.push(' ');
            }
            continue;
        }
        out.push_str(word);
        if words.peek().is_some() {
            out.push(' ');
        }
    }
    out
}

/// `Lua((MOD:X)).SpawnsValues(RemoteSpawn.lua:353)` -> the file and the line.
fn lua_frame_place(line: &str) -> Option<(String, usize)> {
    if !line.starts_with("Lua(") {
        return None;
    }
    let inner = line.rsplit_once('(')?.1.strip_suffix(')')?;
    let (file, number) = inner.rsplit_once(':')?;
    file.ends_with(".lua").then(|| number.parse().ok().map(|n| (file.to_string(), n))).flatten()
}

/// One card. Every sentence on it has to follow from the line it was made of: what the line
/// cannot show (who caused it, what else broke, what will happen next time) is not said.
fn rule_card(counter: usize, line: &str, code: &str, params: BTreeMap<String, String>, title: &str, explanation: &str, action: &str, severity: Severity) -> TranslatedErrorPayload {
    TranslatedErrorPayload {
        called_from: Vec::new(),
        id: format!("err_{}", counter),
        code: code.to_string(),
        params,
        raw_error: line.to_string(),
        source_file: None,
        line_number: None,
        title: title.to_string(),
        explanation: explanation.to_string(),
        recommended_action: action.to_string(),
        mod_name: None,
        severity,
        polyfill_rule_id_suggestion: None,
        own_mod: false,
        own_file: None,
        frame_mod: None,
        log: None,
        log_name: None,
    }
}

/// The game's own noise: lines it writes with no mod at fault and nothing to do about.
/// - `AdvancedAnimator$1.visitFileFailed` + `NoSuchFileException ...\AnimSets|actiongroups`: the
///   game lists those two folders in every active mod and logs each mod that has none (42.22
///   `AdvancedAnimator.searchFolders`); a mod's name in that path is not a fault of the mod.
/// - `BufferOverflowException at PathfindNative.findPath`: the native pathfinder's own buffer.
fn is_game_noise(line: &str) -> bool {
    line.contains("AdvancedAnimator$1.visitFileFailed")
        || (line.contains("NoSuchFileException") && (line.contains("AnimSets") || line.contains("actiongroups")))
        || (line.contains("BufferOverflowException") && line.contains("PathfindNative"))
}

/// `attempted index: getName of non-table: null` -> `getName`.
fn nil_index_key(line: &str) -> Option<String> {
    let rest = line.split("attempted index: ").nth(1)?;
    let (key, tail) = rest.split_once(' ')?;
    (tail.starts_with("of non-table") && !key.is_empty()).then(|| key.to_string())
}

fn translate_log_error_rule(line: &str, counter: usize) -> Option<TranslatedErrorPayload> {
    // Lines the game writes at LOG level for others: ZombieBuddy saying a mod's Java patch was not
    // applied, a Java mod saying it switched itself off. The game itself carries on, the mod does not.
    if let Some(failure) = zb_transform_failure(line) {
        let (class, why) = failure.split_once(": ").unwrap_or((failure, ""));
        let short = class.rsplit('.').next().unwrap_or(class);
        // No source file: the card is about a mod's jar, which nothing in the Studio edits (a
        // `.java` name here put a "Patch IsoZombie.java" button on it that opened the Merger on a
        // mod with no such file).
        let card = rule_card(
            counter,
            line,
            "java_patch_failed",
            crate::catalog::issue_params(&[("class", class), ("short", short), ("why", why), ("variant", if class.starts_with("zombie.") { "" } else { "mod" })]),
            &format!("A Java patch to {} could not be applied", short),
            &format!(
                "A mod's Java change to {}{} no longer fits: the method it changes is different now, usually after an update (ZombieBuddy said: \"{}\"). That change is missing in game; the game itself carries on. Mods with a change to that method: {}. Other mods' changes to {} that came after it can be missing too.",
                short, if class.starts_with("zombie.") { ", part of the game" } else { ", part of another mod's code" }, why, "(see the log)", short
            ),
            "Nothing to do to play. If what the named mod does matters to you, keep it and wait for its author to update it (Save report writes the note for them); if not, switch it off, and the other mods' changes to this class apply. When no mod is named, the Studio could not tell which of those it is.",
            Severity::Exception,
        );
        return Some(card);
    }
    if let Some((tag, what)) = java_mod_turned_off(line) {
        return Some(rule_card(
            counter,
            line,
            "java_mod_turned_off",
            crate::catalog::issue_params(&[("mod", &tag), ("tag", &tag), ("what", &what)]),
            &format!("{} switched itself off", tag),
            &format!("{} wrote \"{}\" (in the log it signs as [{}]): it stopped its own work for the rest of this session. The game carries on without it.", tag, what, tag),
            "If it happens every session, it does not work with what is installed now, usually after an update of the game or of the mod it builds on. Switch it off until its author updates it; its Workshop page often already says so.",
            Severity::Exception,
        ));
    }
    if let Some((host, addon, what)) = java_addon_incompatible(line) {
        let mut card = rule_card(
            counter,
            line,
            "java_addon_incompatible",
            crate::catalog::issue_params(&[("mod", &addon), ("tag", &addon), ("host", &host), ("what", &what)]),
            &format!("{} does not fit this version of {}", addon, host),
            &format!("{} says its addon [{}] does not fit it: \"{}\". The shaders that addon changes do not build, and {} may switch itself off for it (a card of its own says so).", host, addon, what, host),
            &format!("Switch the addon off until its author updates it for this version of {}; {} works without it.", host, host),
            Severity::Exception,
        );
        card.frame_mod = Some(host);
        return Some(card);
    }
    if let Some((tag, undefined)) = java_mod_shader_failed(line) {
        let name = undefined.clone().unwrap_or_default();
        return Some(rule_card(
            counter,
            line,
            "java_shader_failed",
            crate::catalog::issue_params(&[("mod", &tag), ("tag", &tag), ("name", &name), ("variant", if undefined.is_some() { "undefined" } else { "" })]),
            &format!("A shader of {} did not compile", tag),
            &match &undefined {
                Some(v) => format!("{} could not build a shader: it uses \"{}\", which was not defined when it compiled. What that shader draws is missing.", tag, v),
                None => format!("{} could not build a shader; the line says why. What that shader draws is missing.", tag),
            },
            "A name a shader expects but nobody defines is usually given by an addon of that mod. If another card says an addon switched itself off this session, that addon is the one to switch off or update.",
            Severity::Warning,
        ));
    }
    // A card says something went wrong. Nothing but a line the game itself marked as trouble.
    let is_trouble = line.starts_with("ERROR")
        || line.starts_with("WARN")
        || line.contains("Exception")
        || line.contains("stack trace")
        || line.contains("Error:")
        || line.contains("OutOfMemoryError");
    if !is_trouble || is_game_noise(line) {
        return None;
    }
    let none = BTreeMap::new;
    if line.contains("OutOfMemoryError") {
        // "Java heap space" is the limit given to the game's Java (-Xmx); any other OutOfMemoryError
        // (direct buffers, native allocation) is memory outside that limit.
        let heap = line.contains("Java heap space");
        Some(rule_card(
            counter,
            line,
            "java_out_of_memory",
            crate::catalog::issue_params(&[("variant", if heap { "heap" } else { "" })]),
            if heap { "The game's Java ran out of the memory it was given" } else { "The game's Java could not get more memory" },
            if heap {
                "Java reported \"Java heap space\": the game used all the memory its Java is allowed (the -Xmx limit). The game usually closes or stops responding after this."
            } else {
                "Java asked for memory and got none (OutOfMemoryError). This one is not the Java heap limit: it is memory outside it, which runs out when the PC itself is short. The game usually closes or stops responding after this."
            },
            if heap {
                "Diagnostics > Game > Memory shows the limit, what the PC has, and lets you raise it for the next start."
            } else {
                "Close other big programs before playing. Diagnostics > Game > Memory shows what the PC had; raising the Java limit does not help with this one."
            },
            Severity::Exception,
        ))
    } else if line.contains("ToggleDoorActual") && line.contains("ThumpState") {
        // 42.22 IsoDoor.Thump: a zombie that can open doors calls ToggleDoor; ToggleDoorActual then
        // asks `player.isLocalPlayer()` with no player, after the door is open and synced; the
        // state machine catches it ("State execute error: ThumpState").
        let mut card = rule_card(
            counter,
            line,
            "door_npc",
            crate::catalog::issue_params(&[("variant", "zombie")]),
            "A zombie opened a door: an error in the game's own door code (B42)",
            "A zombie opened a door, and the game's Java (IsoDoor.ToggleDoorActual) then asked the player who opened it whether they are the local one, to play the tension music. A zombie is no player, so that question fails. The door is already open when it does: only that music cue and the door's opening animation are skipped. The failing code is the game's own.",
            "Nothing to do: the door opens as it should.",
            Severity::Exception,
        );
        card.source_file = Some("zombie/iso/objects/IsoDoor.java".to_string());
        Some(card)
    } else if line.contains("BufferOverflowException") {
        // One known case: IsoDeadBody.getItem packs the whole body with all it carries
        // (`storeInByteData`), and a body with a bag of fluid containers outgrows the buffer.
        Some(rule_card(
            counter,
            line,
            "item_too_big",
            none(),
            "The game ran out of room packing something into bytes",
            "The game packs things into a buffer of bytes (an item to save it, or an object turned into an item) and this one held more than the buffer has room for. One known case is a mod asking a body on the ground for its item (IsoDeadBody:getItem), which packs the whole body with everything it carries. The lines under this one in the log say which code asked.",
            "If a mod is named next to this line, that mod is where to look. On its own this line does not say whether anything was lost.",
            Severity::Exception,
        ))
    } else if line.contains("ToggleDoorActual") {
        let mut card = rule_card(
            counter,
            line,
            "door_npc",
            none(),
            "The game's own door code met a missing player (B42)",
            "The game's Java for doors (IsoDoor.ToggleDoorActual) asked a player that does not exist whether it is the local one. That happens when something that is not a player opens a door. The failing code is the game's own.",
            "Nothing to patch from here: the code that fails is Java. If it floods the log, the lines under it say what moved through the door.",
            Severity::Exception,
        );
        card.source_file = Some("zombie/iso/objects/IsoDoor.java".to_string());
        Some(card)
    } else if line.contains("BWORoomPrograms") && line.contains("attempted index") {
        let mut card = rule_card(
            counter,
            line,
            "bandits_room",
            none(),
            "Bandits Week One: a room program read the room of a tile that has none",
            "Bandits Week One's BWORoomPrograms.lua asks a tile for its room and then for the room's name without checking that the tile has a room; outdoors it has none, and the script stops there.",
            "Its author is the one to fix it (a missing check for no room). If it repeats and bothers you, switching the mod off is the way to tell whether it matters in your game.",
            Severity::Exception,
        );
        card.source_file = Some("media/lua/shared/BWORoomPrograms.lua".to_string());
        Some(card)
    } else if line.contains("Translator.reportMissingArgumentsFromPastAbuse") {
        // 42.22 Translator.reportMissingArgumentsFromPastAbuse catches the format exception, logs
        // it as a warning and returns the text unformatted.
        let missing = line.contains("Missing arguments");
        let mut card = if missing {
            rule_card(
                counter,
                line,
                "translator_text",
                crate::catalog::issue_params(&[("variant", "missing")]),
                "A text has a placeholder and was given no value",
                "A translated text has a placeholder (%1, %s) and the code that asked for it passed no value. The game shows the text with the placeholder still in it and carries on. The line names the text's key; the key can be the game's own.",
                "Nothing to do to play. Save report names the key and, when a mod ships it, the mod and the file.",
                Severity::Warning,
            )
        } else {
            rule_card(
                counter,
                line,
                "translator_text",
                none(),
                "A text has a % sign the game cannot format",
                "A translated text has a % that is not written %%, so Java's text formatting cannot read it. The game shows the text as it is and carries on. The line names the text's key; the key can be the game's own.",
                "Nothing to do to play. When a mod ships the text, the fix is in its translation file (% written as %%); Save report names the mod, the file and the line.",
                Severity::Warning,
            )
        };
        card.source_file = None;
        Some(card)
    } else if let Some(method) = ui_method_of_nil(line) {
        // `attempted index: prerender of non-table: null`: `ISPanelJoypad.prerender(self)` with
        // the base class nil. Seen for a few frames while the game loads its Lua again; the same
        // line every frame means the class never exists.
        Some(rule_card(
            counter,
            line,
            "ui_draw_reload",
            crate::catalog::issue_params(&[("method", method)]),
            "A window ran before the class it is built on existed",
            &format!("A window ran its {} and the class it is built on was not there at that moment. This is usual for a frame or two while the game loads its Lua again (a save loading, back to the menu). If the same line repeats every frame, the class never exists and the window is broken.", method),
            "A few of these around a reload: nothing to do. Hundreds of them: the mod named next to this line is where to look.",
            Severity::Warning,
        ))
    } else if let Some(key) = nil_index_key(line) {
        Some(rule_card(
            counter,
            line,
            "nil_table",
            crate::catalog::issue_params(&[("key", &key)]),
            "A script used something that was not there (nil)",
            &format!("A script read or called \"{}\" on a value that was nil at that moment. The file and the line are in the stack next to this line in the log.", key),
            "If a mod is named next to this line, that mod's file is where to look. The line alone does not say what stopped working.",
            Severity::Exception,
        ))
    } else if line.contains("NullPointerException") && line.contains("InventoryItem.load(") && line.contains("is null") {
        // 42.22 HandWeapon.load calls item.load on a part with no null check; InventoryItem.loadItem
        // catches the NPE and drops the item being loaded: the weapon that carried the part.
        Some(rule_card(
            counter,
            line,
            "save_item_missing",
            none(),
            "An item in the save could not be loaded",
            "Loading the save, the game met an item it cannot create any more (the line just above, \"cannot create item\", names it and its mod). When that item is a part attached to a weapon, the game drops the whole weapon that carried it, not only the part.",
            "If it matters: leave without saving, or restore a backup of the save, and switch the mod named on the line above back on before loading again. Once the game saves, the item is gone.",
            Severity::Exception,
        ))
    } else if line.contains("NullPointerException") && line.contains("ConnectToServerState") {
        // 42.22 ConnectToServerState.CheckMods calls GameClient.connection.forceDisconnect on a
        // connection that is already gone.
        Some(rule_card(
            counter,
            line,
            "connection_lost",
            none(),
            "The connection to the server was gone while joining",
            "While joining, the game tried to use its connection to the server (to report a missing mod, in the case seen) and the connection was already closed. It is the game's own code, not a mod's.",
            "Join again. If a \"required mod ... not found\" line comes back just before this one, that mod is not installed for this game.",
            Severity::Exception,
        ))
    } else if let Some((tag, what)) = java_mod_own_error(line) {
        // A line a Java mod wrote itself about an exception it caught ("[BloodFX] drawing
        // disabled: java.lang.NullPointerException ...").
        let mut params = BTreeMap::new();
        params.insert("mod".to_string(), tag.clone());
        params.insert("what".to_string(), what.clone());
        let mut card = rule_card(
            counter,
            line,
            "java_mod_error",
            params,
            &format!("{} wrote an error of its own to the log", tag),
            &format!("{} wrote this line itself: \"{}\". It caught the error and went on.", tag, what),
            "Nothing to do: the mod handled it itself, and the line says how. If what the line names is missing in game, Save report makes a text for its author (the Studio cannot change a mod's Java).",
            Severity::Warning,
        );
        card.mod_name = Some(tag);
        Some(card)
    } else if line.contains("NullPointerException") {
        Some(rule_card(
            counter,
            line,
            "java_npe",
            none(),
            "Java met a missing value (NullPointerException)",
            "Some Java code used a value that was not there. The class named after \"at\" on the line says where. The line alone does not say whether a mod caused it: the lines under it in the log do.",
            "Nothing to apply from here. If it repeats and something misbehaves in game, the lines under it name who called that code.",
            Severity::Exception,
        ))
    } else if line.contains("se.krka.kahlua.vm.KahluaException") {
        // `KahluaException: <msg> at BaseLib.error`: a script calling error("msg").
        Some(rule_card(
            counter,
            line,
            "kahlua",
            none(),
            "A Lua script raised an error itself",
            "A script called error(\"...\") with the message on this line. When that happens inside pcall the script caught it itself and nothing broke; the lines around it in the log say which.",
            "Nothing to do unless something misbehaves in game; then the lines next to this one name the file.",
            Severity::Exception,
        ))
    } else {
        None
    }
}

/// Terminates a running Project Zomboid instance using OS process signals or taskkill.
pub fn kill_pz_game(pid: Option<u32>, force: bool) -> Result<String, String> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        let mut cmd = Command::new("taskkill");
        if force {
            cmd.arg("/F");
        }
        if let Some(p) = pid {
            // Only ever the game: a stale or mistaken PID must not close another program.
            cmd.args(&["/FI", "IMAGENAME eq ProjectZomboid64.exe", "/PID", &p.to_string()]);
        } else {
            cmd.args(&["/IM", "ProjectZomboid64.exe"]);
        }
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW

        let output = cmd.output().map_err(|e| format!("Failed to execute taskkill: {}", e))?;
        if output.status.success() {
            let msg = String::from_utf8_lossy(&output.stdout).trim().to_string();
            Ok(if msg.is_empty() { "Project Zomboid process terminated successfully.".to_string() } else { msg })
        } else {
            let err = String::from_utf8_lossy(&output.stderr).trim().to_string();
            // The message is in Windows' language: ask whether the game still runs instead.
            if find_running_pz_pid().is_none() {
                Ok("Project Zomboid is not running.".to_string())
            } else {
                Err(format!("Taskkill error: {}", err))
            }
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        Err("Process termination is only supported on Windows in this build.".to_string())
    }
}

/// Launches ProjectZomboid64.exe directly with custom CLI flags.
pub fn launch_pz_direct(
    pz_install_dir: &str,
    user_zomboid_dir: &str,
    debug: bool,
    windowed: bool,
    nosteam: bool,
    extra_args: &[String],
) -> Result<u32, String> {
    let install_path = Path::new(pz_install_dir);
    let exe_path = install_path.join("ProjectZomboid64.exe");

    if !exe_path.exists() {
        return Err(format!("ProjectZomboid64.exe not found at: {}", exe_path.display()));
    }

    if windowed {
        configure_options_ini(user_zomboid_dir, true);
    }
    sync_java_agents_for_launch(install_path, user_zomboid_dir);
    forget_mp_test_automation(user_zomboid_dir);

    let mut cmd = Command::new(&exe_path);
    cmd.current_dir(install_path);

    if debug {
        cmd.arg("-debug");
    }
    if nosteam {
        cmd.arg("-nosteam");
    }
    for arg in extra_args {
        cmd.arg(arg);
    }
    if !extra_args.iter().any(|a| a == "-modfolders") {
        cmd.args(crate::compat::launch_args(user_zomboid_dir));
    }

    cmd.stdin(std::process::Stdio::null());
    cmd.stdout(std::process::Stdio::null());
    cmd.stderr(std::process::Stdio::null());

    let child = cmd.spawn().map_err(|e| format!("Failed to spawn ProjectZomboid64.exe: {}", e))?;
    let child_pid = child.id();
    thread::sleep(Duration::from_millis(300));
    Ok(find_running_pz_pid().unwrap_or(child_pid))
}

/// Writes an action/command to the IPC command queue file for the companion mod to consume.
pub fn write_game_ipc_command(user_zomboid_dir: &str, command: serde_json::Value) -> Result<String, String> {
    let lua_dir = Path::new(user_zomboid_dir).join("Lua");
    let _ = std::fs::create_dir_all(&lua_dir);

    let json_text = serde_json::to_string_pretty(&command)
        .map_err(|e| format!("Failed to serialize IPC command: {}", e))?;

    // Primary target: Zomboid/Lua/pz_ipc_queue.json (read by PZ getFileReader)
    let ipc_path_lua = lua_dir.join("pz_ipc_queue.json");
    std::fs::write(&ipc_path_lua, &json_text)
        .map_err(|e| format!("Failed to write IPC command queue at {}: {}", ipc_path_lua.display(), e))?;

    // Secondary fallback: Zomboid/pz_ipc_queue.json
    let ipc_path_root = Path::new(user_zomboid_dir).join("pz_ipc_queue.json");
    let _ = std::fs::write(&ipc_path_root, &json_text);

    Ok(format!("IPC command successfully enqueued at {}", ipc_path_lua.display()))
}

/// Reads the latest response emitted by the companion mod from pz_ipc_resp.json.
pub fn read_game_ipc_response(user_zomboid_dir: &str) -> Result<Option<serde_json::Value>, String> {
    let resp_path_lua = Path::new(user_zomboid_dir).join("Lua").join("pz_ipc_resp.json");
    let resp_path_root = Path::new(user_zomboid_dir).join("pz_ipc_resp.json");

    let target_path = if resp_path_lua.exists() {
        resp_path_lua
    } else if resp_path_root.exists() {
        resp_path_root
    } else {
        return Ok(None);
    };

    let content = std::fs::read_to_string(&target_path)
        .map_err(|e| format!("Failed to read IPC response: {}", e))?;

    if content.trim().is_empty() || content.trim() == "{}" {
        return Ok(None);
    }

    let parsed: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse IPC response JSON: {}", e))?;

    Ok(Some(parsed))
}

/// Canonical bridge script from mods/Z_PZModStudio_Bridge, compiled in.
const BRIDGE_CLIENT_LUA: &str =
    include_str!("../../../mods/Z_PZModStudio_Bridge/media/lua/client/PZModStudio_Bridge.lua");

/// True when the bridge in `<user_zomboid_dir>/mods` is missing or differs from the scripts
/// bundled in this build (so fixes reach existing installs, not only new ones).
pub fn bridge_companion_mod_needs_update(user_zomboid_dir: &str) -> bool {
    let b42 = Path::new(user_zomboid_dir).join("mods").join("Z_PZModStudio_Bridge").join("42").join("media").join("lua");
    let same = |path: std::path::PathBuf, expected: &str| {
        std::fs::read_to_string(path).is_ok_and(|current| current == expected)
    };
    !(same(b42.join("client").join("PZModStudio_Bridge.lua"), BRIDGE_CLIENT_LUA)
        && same(b42.join("shared").join("Z_PZModStudio_Polyfills.lua"), crate::patch_generator::MASTER_POLYFILL_LUA)
        && !b42.join("server").join("PZModStudio_Bridge_Server.lua").exists())
}

/// Installs (or refreshes) the Z_PZModStudio_Bridge companion mod in the user's Zomboid/mods directory.
pub fn install_bridge_companion_mod(user_zomboid_dir: &str) -> Result<String, String> {
    let target_mod_dir = Path::new(user_zomboid_dir)
        .join("mods")
        .join("Z_PZModStudio_Bridge");

    let client_lua_dir = target_mod_dir.join("media").join("lua").join("client");
    let b42_dir = target_mod_dir.join("42");
    std::fs::create_dir_all(&client_lua_dir)
        .map_err(|e| format!("Failed to create directories for bridge mod: {}", e))?;
    std::fs::create_dir_all(&b42_dir)
        .map_err(|e| format!("Failed to create 42 directory for bridge mod: {}", e))?;

    let mod_info_content = r#"name=PZ Mod Studio Live Bridge
id=Z_PZModStudio_Bridge
description=Companion IPC Bridge for PZ Mod Studio and AI Agents to execute real-time game commands, equip items, and debug live sessions.
poster=poster.png
icon=icon.png
modversion=1.0.0
versionMin=41.00
"#;

    let b42_mod_info_content = r#"name=PZ Mod Studio Live Bridge
id=Z_PZModStudio_Bridge
description=Companion IPC Bridge for PZ Mod Studio and AI Agents to execute real-time game commands, equip items, and debug live sessions.
poster=poster.png
icon=icon.png
modversion=1.0.0
versionMin=42.0.0
"#;

    let lua_script_content = BRIDGE_CLIENT_LUA;

    let shared_lua_dir = target_mod_dir.join("media").join("lua").join("shared");
    let _ = std::fs::create_dir_all(&shared_lua_dir);

    let b42_media_dir = b42_dir.join("media");
    let b42_client_dir = b42_media_dir.join("lua").join("client");
    let b42_shared_dir = b42_media_dir.join("lua").join("shared");
    let _ = std::fs::create_dir_all(&b42_client_dir);
    let _ = std::fs::create_dir_all(&b42_shared_dir);

    // Older versions also installed the client script under server/, which made it load twice
    // in single player (two consumers racing for the same queue file). Remove those copies.
    for server_dir in [target_mod_dir.join("media").join("lua").join("server"), b42_media_dir.join("lua").join("server")] {
        let _ = std::fs::remove_file(server_dir.join("PZModStudio_Bridge_Server.lua"));
        let _ = std::fs::remove_dir(&server_dir); // only if now empty
    }

    let polyfills_content = crate::patch_generator::generate_master_polyfill_lua();
    let _ = std::fs::write(shared_lua_dir.join("Z_PZModStudio_Polyfills.lua"), &polyfills_content);
    let _ = std::fs::write(b42_shared_dir.join("Z_PZModStudio_Polyfills.lua"), &polyfills_content);

    let _ = std::fs::write(target_mod_dir.join("mod.info"), mod_info_content);
    let _ = std::fs::write(b42_dir.join("mod.info"), b42_mod_info_content);
    let _ = std::fs::write(client_lua_dir.join("PZModStudio_Bridge.lua"), lua_script_content);
    let _ = std::fs::write(b42_client_dir.join("PZModStudio_Bridge.lua"), lua_script_content);

    let png_bytes = crate::patch_generator::get_preview_png_bytes();
    let _ = std::fs::write(target_mod_dir.join("poster.png"), &png_bytes);
    let _ = std::fs::write(target_mod_dir.join("icon.png"), &png_bytes);
    let _ = std::fs::write(b42_dir.join("poster.png"), &png_bytes);
    let _ = std::fs::write(b42_dir.join("icon.png"), &png_bytes);

    let meta = serde_json::json!({
        "is_packaged": true,
        "is_visible_in_modlist": true,
        "created_at": std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs().to_string()).unwrap_or_default(),
        "packaged_mod_ids": [],
        "merged_file_paths": []
    });
    let _ = std::fs::write(target_mod_dir.join("patch_metadata.json"), serde_json::to_string_pretty(&meta).unwrap_or_default());

    Ok(format!("Bridge mod successfully installed at {}", target_mod_dir.display()))
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogFileInfo {
    pub file_name: String,
    pub absolute_path: String,
    pub size_bytes: u64,
    pub modified_timestamp: u64,
    pub is_active_console: bool,
}

pub fn list_available_log_files(user_zomboid_dir: &str) -> Vec<LogFileInfo> {
    let mut files = Vec::new();
    let mut seen = std::collections::HashSet::new();

    let user_dirs = crate::load_order::mod_info::get_all_user_zomboid_dirs(user_zomboid_dir);
    for u_dir in user_dirs {
        // 1. Check console.txt
        let console_p = u_dir.join("console.txt");
        if console_p.exists() && seen.insert(console_p.clone()) {
            let meta = std::fs::metadata(&console_p).ok();
            let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
            let mod_time = meta.and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);

            files.push(LogFileInfo {
                file_name: "console.txt".to_string(),
                absolute_path: console_p.to_string_lossy().to_string(),
                size_bytes: size,
                modified_timestamp: mod_time,
                is_active_console: true,
            });
        }

        // The Studio's dedicated server writes its own log beside it: a server that did not start
        // is read there ("Open the last session" takes whichever of the two is newer).
        let server_p = u_dir.join("server-console.txt");
        if server_p.exists() && seen.insert(server_p.clone()) {
            let meta = std::fs::metadata(&server_p).ok();
            files.push(LogFileInfo {
                file_name: "server-console.txt".to_string(),
                absolute_path: server_p.to_string_lossy().to_string(),
                size_bytes: meta.as_ref().map(|m| m.len()).unwrap_or(0),
                modified_timestamp: meta.and_then(|m| m.modified().ok()).and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok()).map(|d| d.as_secs()).unwrap_or(0),
                is_active_console: false,
            });
        }

        // 2. Check Logs/ folder
        let logs_dir = u_dir.join("Logs");
        if logs_dir.exists() {
            if let Ok(entries) = std::fs::read_dir(logs_dir) {
                for entry in entries.flatten() {
                    let p = entry.path();
                    if p.is_file() && p.extension().map(|e| e == "txt").unwrap_or(false) {
                        if seen.insert(p.clone()) {
                            let meta = std::fs::metadata(&p).ok();
                            let size = meta.as_ref().map(|m| m.len()).unwrap_or(0);
                            let mod_time = meta.and_then(|m| m.modified().ok())
                                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                                .map(|d| d.as_secs())
                                .unwrap_or(0);

                            let fname = p.file_name().unwrap_or_default().to_string_lossy().to_string();
                            files.push(LogFileInfo {
                                file_name: fname,
                                absolute_path: p.to_string_lossy().to_string(),
                                size_bytes: size,
                                modified_timestamp: mod_time,
                                is_active_console: false,
                            });
                        }
                    }
                }
            }
        }
    }

    files.sort_by(|a, b| b.modified_timestamp.cmp(&a.modified_timestamp));
    files
}

pub fn read_log_file(file_path: &str, max_lines: Option<usize>) -> Result<Vec<String>, String> {
    let path = Path::new(file_path);
    if !path.exists() {
        return Err(format!("File does not exist: {}", file_path));
    }

    if let Some(limit) = max_lines {
        return tail_lines(path, limit).map_err(|e| format!("Error opening file: {}", e));
    }
    let file = File::open(path).map_err(|e| format!("Error opening file: {}", e))?;
    Ok(lossy_lines(BufReader::new(file)).collect())
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_line_that_is_not_utf8_does_not_end_the_log() {
        let log: &[u8] = b"LOG  : first\r\nERROR: Gen\xe9rico broke\nWARN : Lua f:0 at Lua((MOD:Late)).File.lua> later\n";
        let lines: Vec<String> = lossy_lines(log).collect();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], "LOG  : first");
        assert!(lines[1].starts_with("ERROR: Gen") && lines[1].ends_with("rico broke"));
        assert!(lines[2].contains("MOD:Late"));
    }

    #[test]
    fn the_tail_of_a_log_is_read_from_its_end() {
        let path = std::env::temp_dir().join(format!("pzms-tail-{}.txt", std::process::id()));
        let mut text = Vec::new();
        for i in 0..50_000 {
            text.extend_from_slice(format!("line {}\r\n", i).as_bytes());
        }
        text.extend_from_slice(b"bad \xff byte\n");
        std::fs::write(&path, &text).unwrap();
        let tail = tail_lines(&path, 3).unwrap();
        assert_eq!(tail, vec!["line 49998".to_string(), "line 49999".to_string(), "bad \u{fffd} byte".to_string()]);
        assert_eq!(tail_lines(&path, 60_000).unwrap().len(), 50_001);
        assert_eq!(read_log_file(&path.to_string_lossy(), Some(2)).unwrap().len(), 2);
        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn bundled_bridge_script_is_valid_lua() {
        let parsed = crate::diff_engine::lua::validate_lua_syntax(BRIDGE_CLIENT_LUA);
        assert!(parsed.is_valid, "bridge Lua does not parse: {:?}", parsed.error_message);
        assert!(BRIDGE_CLIENT_LUA.contains("PRIVILEGED_ACTIONS"));
        for (name, lua) in [
            ("ClaudeAgent.lua", include_str!("../../../mods/Z_ClaudeAgent/42/media/lua/client/ClaudeAgent.lua")),
            ("PZModStudio_Link.lua", include_str!("../../../mods/Z_StudioLink/42/media/lua/client/PZModStudio_Link.lua")),
        ] {
            let parsed = crate::diff_engine::lua::validate_lua_syntax(lua);
            assert!(parsed.is_valid, "{} does not parse: {:?}", name, parsed.error_message);
        }
    }

    #[test]
    fn install_refreshes_stale_bridge_and_removes_server_copy() {
        let dir = std::env::temp_dir().join(format!("pzms-bridge-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let dir_str = dir.to_string_lossy().to_string();
        let server_copy = dir.join("mods/Z_PZModStudio_Bridge/42/media/lua/server/PZModStudio_Bridge_Server.lua");
        std::fs::create_dir_all(server_copy.parent().unwrap()).unwrap();
        std::fs::write(&server_copy, "-- old").unwrap();

        assert!(bridge_companion_mod_needs_update(&dir_str));
        install_bridge_companion_mod(&dir_str).unwrap();
        assert!(!server_copy.exists());
        assert!(!bridge_companion_mod_needs_update(&dir_str));
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[cfg(test)]
mod blame_tests {
    use super::*;

    #[test]
    fn an_addon_its_host_calls_incompatible_is_one_card_under_the_addon() {
        let mut maker = CardMaker::new();
        let mut cards = Vec::new();
        for line in [
            "LOG  : General      f:0> [Viewpoint] model program: [ViewpointDoorLighting] Incompatible model shader; missing: vLight = texelFetch(uDraws, record + LIGHT);",
            "LOG  : General      f:0> [Viewpoint] vehicle program: [ViewpointDoorLighting] Incompatible model shader; missing: vLight = texelFetch(uDraws, record + LIGHT);",
            "LOG  : General      f:0> done",
        ] {
            cards.extend(maker.line(line));
        }
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].code, "java_addon_incompatible");
        assert_eq!(cards[0].mod_name.as_deref(), Some("ViewpointDoorLighting"));
        assert_eq!(cards[0].params.get("host").map(String::as_str), Some("Viewpoint"));
        assert_eq!(cards[0].severity, Severity::Exception);
    }

    #[test]
    fn java_mods_that_stop_working_are_carded_under_their_list_name() {
        let dir = std::env::temp_dir().join(format!("pzms_jar_blame_{}", std::process::id()));
        let jar_dir = dir.join("mods").join("ViewpointLight").join("42").join("media").join("java").join("client");
        std::fs::create_dir_all(&jar_dir).unwrap();
        std::fs::write(dir.join("mods").join("ViewpointLight").join("42").join("mod.info"), "name=Viewpoint - Light Fix\nid=ViewpointLight\n").unwrap();
        let body_dir = dir.join("mods").join("RealisticBodyweight").join("42").join("media").join("java");
        std::fs::create_dir_all(&body_dir).unwrap();
        std::fs::write(dir.join("mods").join("RealisticBodyweight").join("42").join("mod.info"), "name=Realistic Bodyweight\n").unwrap();
        let other_dir = dir.join("mods").join("EFZ").join("42").join("media").join("java");
        std::fs::create_dir_all(&other_dir).unwrap();
        std::fs::write(dir.join("mods").join("EFZ").join("42").join("mod.info"), "name=Other Firearms\n").unwrap();
        let classpath = |p: &std::path::Path, jar: &str| format!("[16-11-26 16:47:46.454] LOG  : General      f:0> [ZB] added to classpath: {}.", p.join(jar).display());
        let lines = vec![
            classpath(&jar_dir, "ViewpointHeadlightPlacement.jar"),
            "[16-11-26 16:47:46.533] LOG  : General      f:0> [ViewpointHeadlightPlacement] Disabled after API error: java.lang.ExceptionInInitializerError.".to_string(),
            classpath(&body_dir, "RealisticBodyweight.jar"),
            "LOG  : General      f:0> [ZB] [!] ERROR transforming zombie.characters.IsoZombie: Cannot assign float arg2 to double.".to_string(),
            "LOG  : General      f:0> java.lang.IllegalStateException: Cannot assign float arg2 to double.".to_string(),
            classpath(&other_dir, "Other.jar"),
            "LOG  : General      f:0> [ZB] [!] ERROR transforming zombie.characters.IsoZombie: Cannot assign float arg2 to double.".to_string(),
            "LOG  : General      f:0> [Viewpoint] light, default shader compile failed (light.frag): lib/spots.glsl(116) : error C1503: undefined variable \"headlightGain\".".to_string(),
            "LOG  : General      f:0> [Viewpoint] air, default shader compile failed (air.frag): lib/spots.glsl(116) : error C1503: undefined variable \"headlightGain\".".to_string(),
            "LOG  : General      f:0> [Viewpoint] disabled after error while preparing:.".to_string(),
            "LOG  : General      f:0> [Viewpoint] crashed while preparing: the crash window shows the report to send.".to_string(),
            classpath(&other_dir, "ViewpointTrueBallistics.jar"),
            "LOG  : General      f:0> [TrueBallistics] WARN Exact player wound location (multiplayer) unavailable: zombie.network.fields.hit.WeaponHit not found; feature disabled".to_string(),
            "LOG  : General      f:0> [TrueBallistics] ERROR True Ballistics disabled: a required game method is missing (game update?). Vanilla firearm combat is used.".to_string(),
            "LOG  : General      f:0> done".to_string(),
        ];
        let mut maker = CardMaker::new();
        let mut cards = Vec::new();
        for line in &lines {
            cards.extend(maker.line(line));
        }
        cards.extend(maker.flush());
        let _ = std::fs::remove_dir_all(&dir);
        let got: Vec<(&str, Option<&str>, &str)> = cards.iter().map(|c| (c.code.as_str(), c.mod_name.as_deref(), c.title.as_str())).collect();
        assert_eq!(
            got,
            vec![
                ("java_mod_turned_off", Some("Viewpoint - Light Fix"), "Viewpoint - Light Fix switched itself off"),
                // The test jars hold no classes: nobody is named rather than the jar loaded last.
                ("java_patch_failed", None, "A Java patch to IsoZombie could not be applied"),
                ("java_shader_failed", Some("Viewpoint"), "A shader of Viewpoint did not compile"),
                ("java_mod_turned_off", Some("Viewpoint"), "Viewpoint switched itself off"),
                // The tag is the end of the jar's name: said by the name in the mod list.
                ("java_mod_turned_off", Some("Other Firearms"), "Other Firearms switched itself off"),
            ]
        );
        assert_eq!(cards[4].params.get("what").map(String::as_str), Some("True Ballistics disabled: a required game method is missing (game update?). Vanilla firearm combat is used"));
        assert_eq!(cards[0].params.get("tag").map(String::as_str), Some("ViewpointHeadlightPlacement"));
        assert_eq!(cards[2].params.get("name").map(String::as_str), Some("headlightGain"));
        assert!(cards.iter().filter(|c| c.code != "java_shader_failed").all(|c| c.severity == Severity::Exception));
    }

    #[test]
    fn an_error_in_the_games_own_file_still_gets_its_card() {
        let mut maker = CardMaker::new();
        let mut cards = Vec::new();
        for _ in 0..2 {
            for line in [
                "ERROR: General      f:0> Lua(Vanilla).getMissingMods> Exception thrown",
                "\tjava.lang.RuntimeException: attempted index: getMods of non-table: null at KahluaThread.tableget(KahluaThread.java:1430).",
                "\t\tLua(Vanilla).getMissingMods(MainScreen.lua:1034)",
                "LOG  : General      f:0> next",
            ] {
                cards.extend(maker.line(line));
            }
        }
        cards.extend(maker.flush());
        assert_eq!(cards.iter().map(|c| c.code.as_str()).collect::<Vec<_>>(), vec!["nil_table"]);
    }

    #[test]
    fn reads_the_mod_the_game_names() {
        let mut blame = ModBlame::default();
        let warn = "WARN : Lua          f:0 at Lua((MOD:Filibuster Rhymes' Certified Used Cars!)).FR_VehicleMechanics.lua> require(\"ISUI/ISVehicleMechanics\") failed";
        assert_eq!(blame.of(warn).as_deref(), Some("Filibuster Rhymes' Certified Used Cars!"));

        // The game also says who replaced each file; that covers lines with only a path.
        blame.observe("LOG  : Mod          f:0> mod \"CleanUI\" overrides media/lua/client/isui/isinventorypane.lua");
        assert_eq!(
            blame.of("ERROR: General  f:0> something broke in media/lua/client/ISUI/ISInventoryPane.lua").as_deref(),
            Some("CleanUI")
        );
        assert_eq!(blame.of("ERROR: General  f:0> media/lua/client/Untouched.lua").as_deref(), None);
        assert_eq!(blame.of("LOG  : General  f:0> nothing to do with mods"), None);
    }


    /// Runs the blame over a real log. PZMS_CONSOLE=C:\\Users\\you\\Zomboid\\console.txt
    /// cargo test blame_on_a_real_log -- --ignored --nocapture
    #[test]
    #[ignore]
    fn blame_on_a_real_log() {
        let path = std::env::var("PZMS_CONSOLE").expect("Set PZMS_CONSOLE to a console.txt");
        let text = std::fs::read_to_string(&path).expect("could not read it");
        let mut blame = ModBlame::default();
        let mut cards: Vec<(String, String)> = Vec::new();
        let mut seen = std::collections::HashSet::new();
        for line in text.lines() {
            blame.observe(line);
            let complaining = line.starts_with("ERROR") || line.starts_with("WARN");
            if let Some(name) = blame.of(line) {
                if complaining && seen.insert(format!("{}|{}", name, line.chars().take(80).collect::<String>())) {
                    let card = blame_card(line, &name, cards.len() + 1);
                    cards.push((card.mod_name.unwrap_or_default(), card.explanation));
                }
            }
        }
        println!("{} file overrides learned from the log", blame.overrides.len());
        println!("{} complaints attributed to a mod:", cards.len());
        for (name, what) in &cards {
            println!("  {name}: {}", what.chars().take(90).collect::<String>());
        }
        assert!(!blame.overrides.is_empty(), "the game logs which mod overrides each file");
    }

    #[test]
    fn a_named_mod_becomes_a_card() {
        let line = "WARN : Lua          f:0 at Lua((MOD:Nature's Call)).NC_Main.lua> require(\"NCUtils\") failed";
        let card = blame_card(line, "Nature's Call", 3);
        assert_eq!(card.mod_name.as_deref(), Some("Nature's Call"));
        assert_eq!(card.source_file.as_deref(), Some("NC_Main.lua"));
        assert!(card.title.contains("Nature's Call"));
        // A failed require says what that means rather than the bare line.
        assert!(card.explanation.contains("asked for the file NCUtils"));
        assert_eq!(card.params.get("variant").map(String::as_str), Some("require"));
        assert!(card.line_number.is_none(), "no line number is invented");
    }
}

#[cfg(test)]
mod card_gate_tests {
    use super::translate_log_error;

    #[test]
    fn a_mod_merely_loading_is_not_an_error() {
        // The line that produced a card claiming Fancy Handwork threw on tick.
        assert!(translate_log_error("LOG  : Mod          f:0> loading FancyHandworkB42_19", 1).is_none());
        assert!(translate_log_error("LOG  : Mod          f:0> loading SaucedCarts", 1).is_none());
        assert!(translate_log_error("LOG  : General      f:0> mod \"dSAG\" overrides media/x.lua", 1).is_none());
    }

    #[test]
    fn a_mods_name_in_a_line_is_not_a_card_of_its_own() {
        // No rule knows a mod by its name: the line gets the card of what it says, if any, and the
        // mod comes from the stack frame the game printed (CardMaker), never from a word in a path.
        assert!(translate_log_error("ERROR: General      f:304> 20)).FHhandleKeybinds> Exception thrown aFancyHandwork.lua:126", 1).is_none());
        // The game listing a folder a mod does not have: its own noise, for every mod.
        for line in [
            "ERROR: General      f:0 st:0> AdvancedAnimator$1.visitFileFailed> Exception thrown",
            "java.nio.file.NoSuchFileException: C:\\Users\\x\\Zomboid\\mods\\FancyHandwork\\common\\media\\AnimSets",
            "java.nio.file.NoSuchFileException: C:\\Users\\x\\Zomboid\\mods\\SaucedCarts\\42\\media\\actiongroups",
            "java.nio.BufferOverflowException at PathfindNative.findPath (Native Method)",
        ] {
            assert!(translate_log_error(line, 1).is_none(), "{}", line);
        }
        let heap = translate_log_error("ERROR: General f:0> GameWindow.uncaughtException> java.lang.OutOfMemoryError: Java heap space", 1).expect("a card");
        assert_eq!((heap.code.as_str(), heap.params.get("variant").map(String::as_str)), ("java_out_of_memory", Some("heap")));
        let native = translate_log_error("java.lang.OutOfMemoryError at MemoryUtil.nmemAllocChecked(MemoryUtil.java:170)", 1).expect("a card");
        assert!(native.explanation.contains("not the Java heap limit"));
        // What a line does say still makes its card, in words that follow from the line.
        let nil = translate_log_error("java.lang.RuntimeException: attempted index: getName of non-table: null at KahluaThread.tableget", 1).expect("a card");
        assert_eq!((nil.code.as_str(), nil.params.get("key").map(String::as_str), nil.polyfill_rule_id_suggestion.as_deref()), ("nil_table", Some("getName"), None));
        let npe = translate_log_error("ERROR: General      f:1> java.lang.NullPointerException: Cannot invoke x at CarController.update", 1).expect("a card");
        assert_eq!((npe.code.as_str(), npe.polyfill_rule_id_suggestion.as_deref()), ("java_npe", None));
        assert!(!npe.explanation.contains("legacy"));
    }

    /// The level the game wrote decides: a WARN is a warning even when it says "Exception".
    #[test]
    fn a_card_carries_the_level_the_game_gave_the_line() {
        use super::Severity;
        let translator = translate_log_error(
            "WARN : General      f:0> Translator.reportMissingArgumentsFromPastAbuse> Exception thrown java.util.UnknownFormatConversionException: Conversion = ')'",
            1,
        )
        .expect("a card");
        assert_eq!(translator.severity, Severity::Warning);
        assert_eq!(translator.code, "translator_text", "a bare % in a mod's text, whatever names it");
        assert_eq!(translator.polyfill_rule_id_suggestion, None);

        // A mod's text with a bare %: the shim does not cover it, so the card does not offer it.
        let text = translate_log_error(
            "WARN : General      f:0> Translator.reportMissingArgumentsFromPastAbuse> Exception thrown java.util.UnknownFormatConversionException: Conversion = ')' at Formatter.parse(null:-1). Message: ERROR: Formatting \"Sandbox_ReactiveSE_EventChance\"",
            3,
        )
        .expect("a card");
        assert_eq!(text.code, "translator_text");
        assert_eq!(text.polyfill_rule_id_suggestion, None);
        assert_eq!(text.source_file, None);

        // A Java mod's own error: named after its tag, no merge package or polyfill offered.
        let java = translate_log_error(
            "LOG  : General      f:3201> [BloodFX] drawing disabled: java.lang.NullPointerException: Cannot read field \"dataid\" because \"this.result\" is null",
            4,
        )
        .expect("a card");
        assert_eq!(java.code, "java_mod_error");
        assert_eq!(java.mod_name.as_deref(), Some("BloodFX"));
        assert_eq!(java.params.get("what").map(String::as_str), Some("drawing disabled"));
        assert_eq!(java.polyfill_rule_id_suggestion, None);
        // A bare NPE line, from nobody in particular, is still the generic card.
        let bare = translate_log_error("LOG  : General      f:3201> java.lang.NullPointerException: Cannot read field", 5).expect("a card");
        assert_eq!(bare.code, "java_npe");

        let threw = translate_log_error("ERROR: General      f:304> java.lang.NullPointerException: Cannot invoke x at CarController.update", 2).expect("a card");
        assert_eq!(threw.severity, Severity::Exception);

        // A stack-trace line has no level prefix: it only exists because something threw.
        assert_eq!(
            Severity::of_line("java.lang.NullPointerException: Cannot invoke \"zombie.input.KeybindId.getId()\""),
            Severity::Exception
        );
        assert_eq!(
            Severity::of_line("WARN : Lua          f:0 at Lua((MOD:The Only Cure)).ZombiesAmputation.lua> require(\"TOC/Controllers/ItemsController\") failed"),
            Severity::Warning
        );
    }
}

#[cfg(test)]
mod live_card_tests {
    /// The cards the Monitor would show for a real log, made the way the Monitor makes them
    /// (`CardMaker`: one card per mod and complaint, the mod from the stack frame).
    /// PZMS_CONSOLE=... cargo test --release the_cards_as_the_monitor_shows_them -- --ignored --nocapture
    #[test]
    #[ignore]
    fn the_cards_as_the_monitor_shows_them() {
        let Ok(path) = std::env::var("PZMS_CONSOLE") else { return };
        let bytes = std::fs::read(&path).expect("console.txt");
        let text = String::from_utf8_lossy(&bytes);
        let mut maker = super::CardMaker::new();
        let mut cards: Vec<super::TranslatedErrorPayload> = Vec::new();
        // A card that comes again with the same id replaces the one shown, as in the app.
        let show = |cards: &mut Vec<super::TranslatedErrorPayload>, mut card: super::TranslatedErrorPayload| {
            super::mark_own_mod_installed(&mut card);
            match cards.iter().position(|c| c.id == card.id) {
                Some(at) => cards[at] = card,
                None => cards.push(card),
            }
        };
        for line in text.lines() {
            for card in maker.line(line) {
                show(&mut cards, card);
            }
        }
        if let Some(card) = maker.flush() {
            show(&mut cards, card);
        }
        let errors = text.lines().filter(|l| l.starts_with("ERROR")).count();
        let warns = text.lines().filter(|l| l.starts_with("WARN")).count();
        println!("{} lines, {} ERROR, {} WARN -> {} cards", text.lines().count(), errors, warns, cards.len());
        for c in &cards {
            println!(
                "[{:?}] {} | mod: {}{} | {}\n        says: {}\n        line: {}",
                c.severity,
                c.code,
                c.mod_name.as_deref().unwrap_or("-"),
                if c.own_mod { format!(" (your mod{})", c.own_file.as_ref().map(|f| format!(", {}:{}", f, c.line_number.unwrap_or(0))).unwrap_or_default()) } else { String::new() },
                c.title,
                c.explanation.chars().take(600).collect::<String>(),
                c.raw_error.trim().chars().take(150).collect::<String>()
            );
        }
    }

    /// How many cards the real log makes, and which lines they come from.
    /// PZMS_CONSOLE=... cargo test --release the_real_cards -- --ignored --nocapture
    #[test]
    #[ignore]
    fn the_real_cards() {
        let Ok(path) = std::env::var("PZMS_CONSOLE") else { return };
        let text = std::fs::read_to_string(&path).expect("console.txt");
        let mut cards = Vec::new();
        let mut from_log_lines = 0;
        for (i, line) in text.lines().enumerate() {
            if let Some(card) = super::translate_log_error(line, i) {
                if !line.starts_with("ERROR") && !line.starts_with("WARN") {
                    from_log_lines += 1;
                }
                cards.push((card.title, line.to_string()));
            }
        }
        let mut titles: Vec<&String> = cards.iter().map(|(t, _)| t).collect();
        titles.sort();
        titles.dedup();
        println!("{} matching lines, {} distinct cards", cards.len(), titles.len());
        for t in &titles {
            let n = cards.iter().filter(|(title, _)| &title == t).count();
            let example = cards.iter().find(|(title, _)| &title == t).unwrap();
            println!("  {:>5}x {}", n, t);
            println!("          from: {}", example.1.trim().chars().take(100).collect::<String>());
        }
        println!("cards made from lines the game did not mark as trouble: {}", from_log_lines);
    }
}

#[cfg(test)]
mod error_stats_tests {
    use super::*;

    #[test]
    fn a_session_is_counted_per_mod_by_what_the_game_did() {
        let lines = [
            "LOG  : Mod          f:0> loading FancyHandworkB42_19",
            "WARN : Lua          f:0 at Lua((MOD:The Only Cure)).ZombiesAmputation.lua> require(\"TOC/Controllers/ItemsController\") failed",
            "\tLua((MOD:Fancy Handwork B42.20)).FHhandleKeybinds(aFancyHandwork.lua:126)",
            "ERROR: General      f:240> 20)).FHhandleKeybinds> Exception thrown",
            "\tLua((MOD:Fancy Handwork B42.20)).FHhandleKeybinds(aFancyHandwork.lua:126)",
            "ERROR: General      f:241> 20)).FHhandleKeybinds> Exception thrown",
            "	java.lang.NullPointerException: Cannot invoke \"zombie.input.KeybindId.getId()\"",
            "	at zombie.core.Core.getKey(Core.java:3023)",
            "LOG  : General      f:242> something else",
            "LOG  : General      f:243> and more",
            "LOG  : General      f:244> and more",
            "ERROR: General      f:0 at ImportedSkeleton.collectBoneFrames  > Could not find bone index for node name: \"Root\"",
        ];
        let stats = error_stats_per_mod(lines.iter().map(|s| s.to_string()));
        assert_eq!(stats.len(), 2, "{:?}", stats);
        assert_eq!(stats[0].mod_name, "Fancy Handwork B42.20");
        assert_eq!(stats[0].exceptions, 2);
        assert_eq!(stats[0].warnings, 0);
        assert_eq!(stats[1].mod_name, "The Only Cure");
        assert_eq!(stats[1].warnings, 1);
        assert_eq!(stats[1].exceptions, 0);
        // Its two ERROR lines and the stack under them (the frame above the first is not its yet).
        assert_eq!(stats[0].lines, 5);
        assert_eq!(stats[1].lines, 1);
    }

    /// Read-only, over a real log: PZMS_LOG=<console.txt or a DebugLog> cargo test real_flood -- --ignored --nocapture
    #[test]
    #[ignore]
    fn real_flood() {
        let Ok(path) = std::env::var("PZMS_LOG") else { return };
        let file = File::open(path).unwrap();
        // A DebugLog line has "[date time] " before what console.txt has.
        let lines = lossy_lines(BufReader::new(file)).map(|l| if l.starts_with('[') { l.split_once("] ").map(|(_, r)| r.to_string()).unwrap_or(l) } else { l });
        let stats = error_stats_per_mod(lines);
        let total: usize = stats.iter().map(|s| s.lines).sum();
        println!("{} lines of trouble", total);
        for s in stats.iter().take(8) {
            println!("{:>6} lines  {:>5} exc {:>5} warn  {}", s.lines, s.exceptions, s.warnings, s.mod_name);
        }
    }
}

#[cfg(test)]
mod one_card_tests {
    use super::CardMaker;

    const BLOCK: [&str; 8] = [
        "STACK TRACE",
        "-----------------------------------------",
        "\tLua((MOD:Take A Bath And Shower [SP/MP])).isValid(TABAS_ISPlace3DItemCursor.lua:48)",
        "\tLua(Vanilla).DoTileBuilding(ISBuildingObject.lua:144)",
        "\tLua((MOD:Project Viewpoint: Visible Construction)).gameDoTileBuilding(VC_Construction.lua:308).",
        "ERROR: General      f:29305> Lua((MOD:Take A Bath And Shower [SP/MP])).isValid> Exception thrown",
        "\tjava.lang.RuntimeException: attempted index: getUnequippedWeight of non-table: null at KahluaThread.tableget(KahluaThread.java:1430).",
        "\tStack trace:",
    ];

    /// The Lua levels of the Java stack under that ERROR, as the game wrote them.
    const STACK: [&str; 4] = [
        "\t\tse.krka.kahlua.vm.KahluaThread.tableget(KahluaThread.java:1430)",
        "\t\tLua((MOD:Take A Bath And Shower [SP/MP])).isValid(TABAS_ISPlace3DItemCursor.lua:48)",
        "\t\tLua(Vanilla).DoTileBuilding(ISBuildingObject.lua:144)",
        "\t\tLua((MOD:Project Viewpoint: Visible Construction)).gameDoTileBuilding(VC_Construction.lua:308)",
    ];

    fn run(maker: &mut CardMaker, lines: &[&str]) -> Vec<super::TranslatedErrorPayload> {
        let mut cards = Vec::new();
        for l in lines {
            cards.extend(maker.line(l));
        }
        cards
    }

    #[test]
    fn a_java_mods_error_is_one_card_under_its_name_without_the_repeat_below_it() {
        let mut maker = CardMaker::new();
        let cards = run(&mut maker, &[
            "LOG  : General      f:3201> [BloodFX] drawing disabled: java.lang.NullPointerException: Cannot read field \"dataid\" because \"this.result\" is null",
            "LOG  : General      f:3201> java.lang.NullPointerException: Cannot read field \"dataid\" because \"this.result\" is null",
            "LOG  : General      f:3201> \tat zombie.core.textures.SmartTexture.getID(SmartTexture.java:389)",
            "LOG  : General      f:3300> java.lang.NullPointerException: something else",
        ]);
        let codes: Vec<_> = cards.iter().map(|c| (c.code.as_str(), c.mod_name.as_deref())).collect();
        assert_eq!(codes, vec![("java_mod_error", Some("BloodFX")), ("java_npe", None)]);
    }

    /// A zombie thumping a door open is the game's own bug, not an NPC mod's; and a mod's caught
    /// BufferOverflowException is the corpse it asked for an item, under that mod's name (42.21 log).
    #[test]
    fn a_zombies_door_and_a_corpse_too_big_to_pack() {
        let mut maker = CardMaker::new();
        let mut cards = run(&mut maker, &[
            "ERROR: Multiplayer  f:262136> StateMachine.stateExecute> Exception thrown",
            "\tjava.lang.NullPointerException: Cannot invoke \"zombie.characters.IsoPlayer.isLocalPlayer()\" because \"player\" is null at IsoDoor.ToggleDoorActual(IsoDoor.java:1620). Message: State execute error: ThumpState",
            "\tStack trace:",
            "\t\tzombie.iso.objects.IsoDoor.ToggleDoorActual(IsoDoor.java:1620)",
            "ERROR: General      f:275597 at KahluaThread.flushErrorMessage      > dumping Lua stack trace",
            "\tLua((MOD:Computer Mod)).pcall(ComputerMod_ComputerTypes.lua:49)",
            "\tLua((MOD:Computer Mod)).getInventoryItem(ComputerMod_ComputerTypes.lua:49)",
            "ERROR: General      f:275597> Lua((MOD:Computer Mod)).pcall> Exception thrown",
            "\tjava.nio.BufferOverflowException at Buffer.nextPutIndex(null:-1).",
            "\tStack trace:",
            "\t\tzombie.iso.objects.IsoDeadBody.getItem(IsoDeadBody.java:693)",
            "LOG  : General      f:275600> done",
        ]);
        cards.extend(maker.flush());
        let got: Vec<_> = cards
            .iter()
            .map(|c| (c.code.as_str(), c.params.get("variant").map(|v| v.as_str()), c.mod_name.as_deref()))
            .collect();
        assert_eq!(got, vec![("door_npc", Some("zombie"), None), ("item_too_big", None, Some("Computer Mod"))]);
    }

    /// The Monitor's cards for this PC's console.txt, folded as the app folds them (title, shape of
    /// the line, mod). PZMS_ZOMBOID_DIR.
    #[test]
    #[ignore]
    fn the_real_sessions_cards() {
        let Ok(dir) = std::env::var("PZMS_ZOMBOID_DIR") else { return };
        let file = std::fs::File::open(std::path::Path::new(&dir).join("console.txt")).expect("console.txt");
        let shape = |l: &str| -> String {
            let l = l.trim_start();
            let rest = if l.starts_with("ERROR") || l.starts_with("WARN") { l.split_once('>').map(|(_, r)| r).unwrap_or(l) } else { l };
            rest.chars().map(|c| if c.is_ascii_digit() { 'N' } else { c }).collect::<String>().trim().chars().take(90).collect()
        };
        let mut maker = CardMaker::new();
        let mut cards: Vec<(super::TranslatedErrorPayload, usize)> = Vec::new();
        let add = |card: super::TranslatedErrorPayload, cards: &mut Vec<(super::TranslatedErrorPayload, usize)>| {
            let same = |c: &super::TranslatedErrorPayload| {
                c.mod_name == card.mod_name && ((c.title == card.title && shape(&c.raw_error) == shape(&card.raw_error)) || (c.code == "translator_text" && card.code == "translator_text"))
            };
            match cards.iter_mut().find(|(c, _)| same(c)) {
                Some((_, n)) => *n += 1,
                None => cards.push((card, 1)),
            }
        };
        for line in super::lossy_lines(std::io::BufReader::new(file)) {
            for card in maker.line(&line) {
                add(card, &mut cards);
            }
        }
        if let Some(card) = maker.flush() {
            add(card, &mut cards);
        }
        for (c, n) in &cards {
            println!("--- {:?} x{} | {} | mod: {:?} | from: {:?}\n    {}\n    {}", c.severity, n, c.code, c.mod_name, c.called_from, c.title, c.raw_error.chars().take(230).collect::<String>());
        }
        println!("{} cards", cards.len());
    }

    #[test]
    fn every_way_java_refuses_a_bare_percent_is_the_same_card() {
        for line in [
            "WARN : General      f:0> Translator.reportMissingArgumentsFromPastAbuse> Exception thrown java.util.UnknownFormatConversionException: Conversion = ')' at Formatter.parse(null:-1). Message: ERROR: Formatting \"Sandbox_X\"",
            "WARN : General      f:0> Translator.reportMissingArgumentsFromPastAbuse> Exception thrown java.util.MissingFormatArgumentException: Format specifier '% A' at Formatter.format(null:-1). Message: ERROR: Missing arguments for \"UI_trait_X\"",
            "WARN : General      f:0> Translator.reportMissingArgumentsFromPastAbuse> Exception thrown java.util.FormatFlagsConversionMismatchException: Conversion = s, Flags =   at Formatter$FormatSpecifier.failMismatch(null:-1). Message: ERROR: Formatting \"UI_X\"",
        ] {
            assert_eq!(super::translate_log_error(line, 1).map(|c| c.code).as_deref(), Some("translator_text"), "{}", line);
        }
    }

    #[test]
    fn an_error_and_the_exception_under_it_are_one_card() {
        let mut maker = CardMaker::new();
        let mut lines = BLOCK.to_vec();
        lines.extend(STACK);
        let mut cards = run(&mut maker, &lines);
        assert!(cards.is_empty(), "held until the block ends");
        cards.extend(maker.line("LOG  : General      f:29306> done"));
        assert_eq!(cards.len(), 1, "{:#?}", cards);
        let card = &cards[0];
        assert_eq!(card.mod_name.as_deref(), Some("Take A Bath And Shower [SP/MP]"));
        assert_eq!(card.called_from, vec!["Project Viewpoint: Visible Construction"]);
        // Said in plain words, where it failed, and the game's own words kept at the end.
        assert!(card.explanation.starts_with("Take A Bath And Shower [SP/MP] asked for getUnequippedWeight of something that does not exist (nil), in TABAS_ISPlace3DItemCursor.lua:48."), "{}", card.explanation);
        assert!(card.explanation.ends_with("java.lang.RuntimeException: attempted index: getUnequippedWeight of non-table: null at KahluaThread.tableget(KahluaThread.java:1430)."), "{}", card.explanation);
        assert_eq!(card.params.get("variant").map(String::as_str), Some("lua"));
        assert_eq!(card.params.get("place").map(String::as_str), Some("TABAS_ISPlace3DItemCursor.lua:48"));
        assert!(card.raw_error.starts_with("ERROR:"));
        assert!(maker.flush().is_none());

        // The same error again (every tick): no new card, and no card for its exception either.
        let again = run(&mut maker, &BLOCK);
        let again_end = maker.line("LOG  : General      f:29307> done");
        assert!(again.is_empty() && again_end.is_empty());
    }

    #[test]
    fn an_error_on_every_frame_is_one_card_with_its_file_and_line() {
        let mut maker = CardMaker::new();
        let mut cards = Vec::new();
        for frame in 156..176 {
            for line in [
                format!("ERROR: General      f:{}> Lua((MOD:Abundant and visible TV remote)).SpawnsValues> Exception thrown", frame),
                "\tjava.lang.RuntimeException: Object tried to call nil in SpawnsValues at KahluaUtil.fail(KahluaUtil.java:100).".to_string(),
                "\tStack trace:".to_string(),
                "\t\tse.krka.kahlua.vm.KahluaUtil.fail(KahluaUtil.java:100)".to_string(),
                "\t\tLua((MOD:Abundant and visible TV remote)).SpawnsValues(RemoteSpawn.lua:353)".to_string(),
                "\t\tLua((MOD:Abundant and visible TV remote)).CleanList(RemoteSpawn.lua:334)".to_string(),
                format!("LOG  : Lua          f:{} st:910,647,{:03}> supr White Tiles .", frame, frame),
            ] {
                cards.extend(maker.line(&line));
            }
        }
        cards.extend(maker.flush());
        assert_eq!(cards.len(), 1, "{:#?}", cards.iter().map(|c| &c.raw_error).collect::<Vec<_>>());
        assert_eq!((cards[0].source_file.as_deref(), cards[0].line_number), (Some("RemoteSpawn.lua"), Some(353)), "the innermost frame, not the caller's");
        assert_eq!(super::without_frame_stamp("ERROR: General      f:156 st:1,234> Lua((MOD:X)).fn> Exception thrown"), "ERROR: General      > Lua((MOD:X)).fn> Exception thrown");
        assert_eq!(super::without_frame_stamp("WARN : Lua f:0 at Lua((MOD:X)).F.lua> require(\"f:9\") failed"), "WARN : Lua at Lua((MOD:X)).F.lua> require(\"f:9\") failed");
    }

    #[test]
    fn a_stack_dump_of_one_error_is_not_the_next_ones() {
        let mut maker = CardMaker::new();
        // An error of the Studio Link with only the game's own code under it, then the dump of
        // an earlier error (other mods) and a different error.
        let cards = run(&mut maker, &[
            "ERROR: General      f:0> Lua((MOD:PZ Mod Studio Link)).report> Exception thrown",
            "\tjava.lang.RuntimeException: attempted index: last of non-table: null at KahluaThread.tableget(KahluaThread.java:1430).",
            "\tStack trace:",
            "\t\tLua((MOD:PZ Mod Studio Link)).report(PZModStudio_Errors.lua:178)",
            "\t\tLua(Vanilla).boom(Apocalypse.lua:2)",
            "ERROR: General      f:0 at KahluaThread.flushErrorMessage > dumping Lua stack trace",
            "STACK TRACE",
            "\tLua((MOD:Some Other Mod)).fn(Other.lua:1)",
            "\tLua((MOD:A Caller)).fn(Caller.lua:2)",
            "LOG  : General      f:1> next",
        ]);
        let link = cards.iter().find(|c| c.mod_name.as_deref() == Some("PZ Mod Studio Link")).expect("its card");
        assert!(link.called_from.is_empty(), "nothing but the game called it: {:?}", link.called_from);
        assert!(link.explanation.contains("The game's words: java.lang.RuntimeException"), "{}", link.explanation);
    }

    #[test]
    fn a_window_drawing_during_the_lua_reload_is_a_warning_with_nothing_to_do() {
        let mut maker = CardMaker::new();
        // Exactly as the game wrote it while a save loaded (frame 0), then its stack dump.
        let cards = run(&mut maker, &[
            "ERROR: General      f:0> Lua((MOD:Mod Update and Alert System)).prerender> Exception thrown",
            "\tjava.lang.RuntimeException: attempted index: prerender of non-table: null at KahluaThread.tableget(KahluaThread.java:1430).",
            "\tStack trace:",
            "\t\tLua((MOD:Mod Update and Alert System)).prerender(chuckleberryFinnModdingAlertSystem.lua:80)",
            "ERROR: General      f:0 at KahluaThread.flushErrorMessage > -------------------------------------------------------------",
            "attempted index: prerender of non-table: null",
            "ERROR: General      f:0 at KahluaThread.flushErrorMessage > dumping Lua stack trace",
            "STACK TRACE",
            "\tLua((MOD:Mod Update and Alert System)).prerender(chuckleberryFinnModdingAlertSystem.lua:80)",
            "LOG  : General      f:0> STATE: exit zombie.gameStates.LoadingQueueState",
        ]);
        assert_eq!(cards.len(), 1, "one card, none for the dump header: {:?}", cards.iter().map(|c| &c.raw_error).collect::<Vec<_>>());
        assert_eq!(cards[0].code, "ui_draw_reload");
        assert_eq!(cards[0].severity, super::Severity::Warning);
        assert_eq!(cards[0].mod_name.as_deref(), Some("Mod Update and Alert System"));
        assert_eq!(cards[0].params.get("method").map(String::as_str), Some("prerender"));
        assert!(cards[0].polyfill_rule_id_suggestion.is_none());
        // A nil table that is not a window's base class is still the plain card.
        assert!(super::ui_method_of_nil("attempted index: getUnequippedWeight of non-table: null").is_none());
        assert_eq!(super::lua_failure_kind("java.lang.RuntimeException: __concat not defined for operands: , and null at KahluaUtil.fail(KahluaUtil.java:100)."), Some(("concat", String::new())));
        assert_eq!(super::lua_failure_kind("java.lang.RuntimeException: attempted index: getName of non-table: null at KahluaThread.tableget(KahluaThread.java:1430)."), Some(("index", "getName".to_string())));
        assert_eq!(super::lua_failure_kind("java.lang.RuntimeException: Object tried to call nil in SpawnsValues at KahluaUtil.fail(KahluaUtil.java:100)."), Some(("call", "SpawnsValues".to_string())));
        assert_eq!(super::lua_failure_kind("java.lang.RuntimeException: __lt not defined for operands at KahluaUtil.fail"), Some(("compare", String::new())));
        assert_eq!(super::lua_failure_kind("java.lang.NullPointerException"), None);
        assert_eq!(super::lua_failure_kind("java.lang.RuntimeException: No implementation found for function: get(class zombie.core.properties.PropertyContainer {1=0}, class zombie.iso.SpriteDetails.IsoFlagType WallN)"), Some(("no_impl", "get|PropertyContainer".to_string())));
    }

    #[test]
    fn a_recursive_require_says_the_files_ask_for_each_other() {
        let mut maker = super::CardMaker::new();
        let mut cards = maker.line("WARN : Lua          f:0 at Lua((MOD:Loose Ends)).GameMaster.lua> recursive require(): D:/Apps/Steam/steamapps/workshop/content/108600/3824292323/mods/LooseEnds/42/media/lua/server/LooseEnds/Delivery.lua.");
        cards.extend(maker.line("LOG  : General      f:1> next"));
        cards.extend(maker.flush());
        let card = cards.iter().find(|c| c.mod_name.as_deref() == Some("Loose Ends")).expect("a card");
        assert_eq!(card.params.get("variant").map(String::as_str), Some("recursive_require"));
        assert_eq!(card.params.get("file").map(String::as_str), Some("server/LooseEnds/Delivery.lua"));
        assert!(card.recommended_action.starts_with("Nothing to do"), "{}", card.recommended_action);
    }

    #[test]
    fn a_held_card_goes_out_when_nothing_more_comes() {
        let mut maker = CardMaker::new();
        assert!(run(&mut maker, &BLOCK[..6]).is_empty());
        let card = maker.flush().expect("the ERROR's card");
        assert!(card.raw_error.starts_with("ERROR:"));
    }

    #[test]
    fn an_exception_with_no_error_above_still_makes_a_card() {
        let mut maker = CardMaker::new();
        let cards = run(&mut maker, &[
            "\tLua((MOD:Fancy Handwork B42.20)).FHhandleKeybinds(aFancyHandwork.lua:126)",
            "\tjava.lang.NullPointerException: Cannot invoke \"zombie.input.KeybindId.getId()\"",
        ]);
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].mod_name.as_deref(), Some("Fancy Handwork B42.20"));
    }
}

#[cfg(test)]
mod blame_window_tests {
    use super::ModBlame;

    #[test]
    fn a_java_exception_is_blamed_on_the_lua_frame_just_above_it() {
        let mut blame = ModBlame::default();
        // Exactly as the game writes it.
        let lines = [
            "\tLua((MOD:Fancy Handwork B42.20)).FHhandleKeybinds(aFancyHandwork.lua:126)",
            "ERROR: General      f:240> 20)).FHhandleKeybinds> Exception thrown",
            "\tjava.lang.NullPointerException: Cannot invoke \"zombie.input.KeybindId.getId()\"",
        ];
        let mut blamed = Vec::new();
        for line in lines {
            blame.observe(line);
            blamed.push(blame.of(line));
        }
        assert_eq!(blamed[1].as_deref(), Some("Fancy Handwork B42.20"));
        assert_eq!(blamed[2].as_deref(), Some("Fancy Handwork B42.20"));
    }

    /// The shim's event guard and the profiler wrap a mod's handler: the mod is to blame.
    #[test]
    fn a_studio_wrapper_under_a_mods_frame_does_not_take_the_blame() {
        let mut blame = ModBlame::default();
        let lines = [
            "\tLua((MOD:Better Cars)).onUpdate(BC_Main.lua:40)",
            "\tLua((MOD:PZ Mod Studio Profiler)).wrapped(PZModStudio_Profiler.lua:61)",
            "ERROR: General      f:240> Profiler)).wrapped> Exception thrown",
        ];
        let mut blamed = Vec::new();
        for line in lines {
            blame.observe(line);
            blamed.push(blame.of(line));
        }
        assert_eq!(blamed[1].as_deref(), Some("Better Cars"));
        assert_eq!(blamed[2].as_deref(), Some("Better Cars"));
    }

    /// A package's copy of a mod file is named as such, not as a mod of its own.
    #[test]
    fn a_packages_copy_is_a_patched_copy_not_a_mod() {
        let card = super::blame_card(
            "WARN : Lua          f:0 at Lua((MOD:PZ Mod Studio Patch: MergedPatch)).MoodlesIn.lua> require(\"LSMoodleManager\") failed",
            "PZ Mod Studio Patch: MergedPatch",
            1,
        );
        assert_eq!(card.title, "MoodlesIn.lua, the patched copy in MergedPatch, reported a problem");
        assert!(card.recommended_action.contains("unmerge"));
        assert_eq!(card.severity, super::Severity::Warning);
        // The other shape the game writes: the function, then the file and line in parentheses.
        let card = super::blame_card(
            "ERROR: General      f:121092> Lua((MOD:PZ Mod Studio Patch: MergedPatch)).HandleZombiesAmputations(ZombiesAmputation.lua:86) > Exception thrown",
            "PZ Mod Studio Patch: MergedPatch",
            2,
        );
        assert_eq!(card.source_file.as_deref(), Some("ZombiesAmputation.lua"));
        assert_eq!(card.title, "ZombiesAmputation.lua, the patched copy in MergedPatch, reported a problem");
    }

    #[test]
    fn a_name_does_not_carry_across_the_whole_log() {
        let mut blame = ModBlame::default();
        blame.observe("\tLua((MOD:Some Mod)).thing(file.lua:1)");
        for _ in 0..5 {
            blame.observe("LOG  : General      f:0> something else entirely");
        }
        assert_eq!(blame.of("ERROR: General      f:9> Exception thrown"), None);
    }

    #[test]
    fn a_connection_lost_while_joining_is_the_games_not_a_merge() {
        // From a real 42.21 join, right after the Workshop downloads.
        let threw = "\tjava.lang.NullPointerException: Cannot invoke \"zombie.core.raknet.UdpConnection.forceDisconnect(String)\" because \"zombie.network.GameClient.connection\" is null at ConnectToServerState.CheckMods(ConnectToServerState.java:674). Message: StateAction Exception thrown. Handled gracefully, returning StateAction.Continue";
        let card = super::translate_log_error(threw, 1).expect("a card");
        assert_eq!(card.code, "connection_lost");
        assert_eq!(card.polyfill_rule_id_suggestion, None, "no merge or polyfill reaches the game's connection");
    }

    #[test]
    fn a_save_item_that_cannot_be_made_is_its_own_card_and_names_the_mod() {
        // From a real 42.21 log: the item the game could not create, then the exception.
        let mut blame = ModBlame::default();
        let made = "LOG  : General      f:0> InventoryItemFactory.CreateItem() cannot create item: TYPE = item,registryID = 1964,fulltype = \"Base.FiberglassStock\",modID = \"VFExpansion1\",existsAsVanilla = false,isModded = true,obsolete = false,removed = true,modOverrides = 0,";
        let threw = "\tjava.lang.NullPointerException: Cannot invoke \"zombie.inventory.InventoryItem.load(java.nio.ByteBuffer, int)\" because \"item\" is null at HandWeapon.load(HandWeapon.java:1473).";
        blame.observe(made);
        blame.observe("ERROR: General      f:0> InventoryItem.loadItem> Exception thrown");
        blame.observe(threw);
        assert_eq!(blame.of(threw).as_deref(), Some("VFExpansion1"));
        let card = super::translate_log_error(threw, 1).expect("a card");
        assert_eq!(card.code, "save_item_missing");
        assert_eq!(card.polyfill_rule_id_suggestion, None);
    }

    #[test]
    fn a_line_in_the_games_own_file_is_not_blamed_on_the_mod_above_it() {
        // Seen in a real 42.21 log: Nomad Grid RV's line, then two of the game's own.
        let mut blame = ModBlame::default();
        let lines = [
            "WARN : Lua          f:0 at Lua((MOD:Nomad Grid RV)).GridRVMoveables.lua> require(\"TimedActions/ISTimedActionQueue\") failed",
            "WARN : Lua          f:0 at Lua(Vanilla).ISAnimalContextMenu    > require(\"BuildingObjects/ISAnimalPickMateCursor\") failed",
            "WARN : Lua          f:0 at Lua(Vanilla).TakeSameType           > require(\"ISUI/InventoryWindow/ISLootWindowControlHandler\") failed",
        ];
        blame.observe(lines[0]);
        assert_eq!(blame.of(lines[0]).as_deref(), Some("Nomad Grid RV"));
        blame.observe(lines[1]);
        assert_eq!(blame.of(lines[1]), None);
        blame.observe(lines[2]);
        assert_eq!(blame.of(lines[2]), None);
        // Nor does the name come back for an anonymous line after them.
        assert_eq!(blame.of("ERROR: General      f:9> Exception thrown"), None);
    }
}

#[cfg(test)]
mod grouped_and_own_tests {
    use super::*;

    fn bare(key: &str) -> String {
        format!("WARN : General      f:0> Translator.reportMissingArgumentsFromPastAbuse> Exception thrown java.util.UnknownFormatConversionException: Conversion = ')' at Formatter.parse(null:-1). Message: ERROR: Formatting \"{}\"", key)
    }

    fn missing(key: &str) -> String {
        format!("WARN : General      f:0> Translator.reportMissingArgumentsFromPastAbuse> Exception thrown java.util.MissingFormatArgumentException: Format specifier '% A' at Formatter.format(null:-1). Message: ERROR: Missing arguments for \"{}\"", key)
    }

    /// Shows a card the way the app does: one that comes again with the same id replaces it.
    fn show(shown: &mut Vec<TranslatedErrorPayload>, cards: Vec<TranslatedErrorPayload>) {
        for card in cards {
            match shown.iter().position(|c| c.id == card.id) {
                Some(at) => shown[at] = card,
                None => shown.push(card),
            }
        }
    }

    #[test]
    fn the_texts_the_game_cannot_format_are_one_card_that_lists_them() {
        let mut maker = CardMaker::new();
        let mut shown = Vec::new();
        for i in 0..13 {
            show(&mut shown, maker.line(&bare(&format!("Sandbox_Key{}", i))));
        }
        assert_eq!(shown.len(), 1, "thirteen texts, one card");
        // The same text again says nothing new: no card is sent.
        assert!(maker.line(&bare("Sandbox_Key0")).is_empty());
        // A placeholder given no value is another kind: its own card.
        show(&mut shown, maker.line(&missing("UI_trait_X")));
        assert_eq!(shown.len(), 2);

        let card = &shown[0];
        assert_eq!((card.code.as_str(), card.severity), ("translator_text", Severity::Warning));
        assert_eq!(card.title, "13 texts have a % sign the game cannot format");
        assert_eq!(card.params["variant"], "many");
        assert_eq!(card.params["count"], "13");
        assert_eq!(card.params["keys"], (0..8).map(|i| format!("Sandbox_Key{}", i)).collect::<Vec<_>>().join(", "));
        assert_eq!((card.params["more"].as_str(), card.params["moreKey"].as_str()), ("5", "and_more"));
        assert!(card.explanation.contains("Sandbox_Key7 and 5 more."), "{}", card.explanation);
        assert!(!card.explanation.contains("Sandbox_Key8"), "{}", card.explanation);
        // No mod is known to ship them (a test sees no installed mods): the card does not name one.
        assert_eq!(card.mod_name, None);
        assert_eq!(card.params["whoseKey"], "text_whose_none_many");
        assert!(card.raw_error.contains("Sandbox_Key0"), "the first line stays the card's line");

        let other = &shown[1];
        assert_eq!(other.title, "A text has a placeholder and was given no value");
        assert_eq!((other.params["variant"].as_str(), other.params["count"].as_str(), other.params["keys"].as_str()), ("missing", "1", "UI_trait_X"));

        // A new session starts the cards over.
        maker.new_session();
        let again = maker.line(&bare("Sandbox_Key0"));
        assert_eq!(again.len(), 1);
        assert_eq!((again[0].params["variant"].as_str(), again[0].params["count"].as_str()), ("", "1"));
    }

    fn error_block(mod_name: &str, file: &str) -> Vec<String> {
        vec![
            format!("ERROR: General      f:156> Lua((MOD:{})).SpawnsValues> Exception thrown", mod_name),
            "\tjava.lang.RuntimeException: Object tried to call nil in SpawnsValues at KahluaUtil.fail(KahluaUtil.java:100).".to_string(),
            "\tStack trace:".to_string(),
            "\t\tse.krka.kahlua.vm.KahluaUtil.fail(KahluaUtil.java:100)".to_string(),
            format!("\t\tLua((MOD:{})).SpawnsValues({}:353)", mod_name, file),
            "LOG  : Lua          f:157> done".to_string(),
        ]
    }

    fn card_of(lines: &[String]) -> TranslatedErrorPayload {
        let mut maker = CardMaker::new();
        let mut cards = Vec::new();
        for l in lines {
            cards.extend(maker.line(l));
        }
        cards.extend(maker.flush());
        assert_eq!(cards.len(), 1, "{:#?}", cards.iter().map(|c| &c.raw_error).collect::<Vec<_>>());
        cards.remove(0)
    }

    #[test]
    fn a_card_about_the_players_own_mod_says_so_and_finds_its_file() {
        let root = std::env::temp_dir().join(format!("pzms-own-card-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        for rel in [
            "common/media/lua/server/RemoteSpawn.lua",
            "42/media/lua/server/RemoteSpawn.lua",
            "42/media/lua/client/Twice.lua",
            "42/media/lua/shared/Twice.lua",
            "41/media/lua/server/Old.lua",
        ] {
            let p = root.join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, "-- x").unwrap();
        }
        let own = vec![OwnMod { name: "My Mod".to_string(), id: "Mine".to_string(), root: root.clone() }];

        // The same path in common/ and in the version folder is one file: the version folder's.
        let mut card = card_of(&error_block("My Mod", "RemoteSpawn.lua"));
        assert_eq!((card.source_file.as_deref(), card.line_number, card.frame_mod.as_deref()), (Some("RemoteSpawn.lua"), Some(353), Some("My Mod")));
        mark_own_mod(&mut card, &own, Some(42_021));
        assert!(card.own_mod);
        assert_eq!(card.own_file.as_deref().map(std::path::Path::new), Some(root.join("42/media/lua/server/RemoteSpawn.lua").as_path()));

        // Two different files of that name: none is given rather than the wrong one.
        let mut card = card_of(&error_block("My Mod", "Twice.lua"));
        mark_own_mod(&mut card, &own, Some(42_021));
        assert!(card.own_mod && card.own_file.is_none());

        // A file only a folder the game does not load has: not given.
        let mut card = card_of(&error_block("My Mod", "Old.lua"));
        mark_own_mod(&mut card, &own, Some(42_021));
        assert!(card.own_mod && card.own_file.is_none());

        // The place came from another mod's frame: the file is not this mod's to open.
        let mut card = card_of(&error_block("My Mod", "RemoteSpawn.lua"));
        card.frame_mod = Some("Somebody Else".to_string());
        mark_own_mod(&mut card, &own, Some(42_021));
        assert!(card.own_mod && card.own_file.is_none());

        // Another mod's card is left as it was.
        let mut card = card_of(&error_block("Somebody Else", "RemoteSpawn.lua"));
        mark_own_mod(&mut card, &own, Some(42_021));
        assert!(!card.own_mod && card.own_file.is_none());

        // The app gets both.
        let mut card = card_of(&error_block("My Mod", "RemoteSpawn.lua"));
        mark_own_mod(&mut card, &own, Some(42_021));
        let json = serde_json::to_value(&card).unwrap();
        assert_eq!(json["own_mod"], true);
        assert!(json["own_file"].as_str().is_some_and(|f| f.ends_with("RemoteSpawn.lua")));
        let _ = std::fs::remove_dir_all(&root);
    }
}

#[cfg(test)]
mod other_log_tests {
    use super::*;

    #[test]
    fn a_servers_log_makes_cards_from_what_is_new_and_starts_over_when_rewritten() {
        let dir = std::env::temp_dir().join(format!("pzms-other-log-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("server-console.txt");
        let error = "	Lua((MOD:Fancy Handwork B42.20)).FHhandleKeybinds(aFancyHandwork.lua:126)
ERROR: General      f:241> 20)).FHhandleKeybinds> Exception thrown
	java.lang.NullPointerException: Cannot invoke \"zombie.input.KeybindId.getId()\"
	at zombie.core.Core.getKey(Core.java:3023)
LOG  : General      f:242> something else
";
        // What was there before the Studio looked is not reported.
        std::fs::write(&path, format!("LOG  : General  old line
{}", error)).unwrap();
        let mut log = OtherLog { kind: "server", offset: None, cards: CardMaker::new() };
        assert!(follow_other_log(&mut log, &path).0.is_empty(), "the first look only learns where the log ends");

        // New lines: an error, then nothing more (the card held for its exception goes out).
        let mut f = std::fs::OpenOptions::new().append(true).open(&path).unwrap();
        std::io::Write::write_all(&mut f, error.as_bytes()).unwrap();
        drop(f);
        let (mut cards, restarted) = follow_other_log(&mut log, &path);
        cards.extend(follow_other_log(&mut log, &path).0);
        assert!(!restarted);
        assert_eq!(cards.len(), 1, "one card for the new error: {:?}", cards.iter().map(|c| &c.title).collect::<Vec<_>>());

        // A new server start rewrites the log from the top.
        std::fs::write(&path, "LOG  : General  started
").unwrap();
        let (cards, restarted) = follow_other_log(&mut log, &path);
        assert!(restarted && cards.is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }
}


#[cfg(test)]
mod require_cards {
    #[test]
    fn a_failed_require_says_the_file_is_not_there_and_that_nothing_is_to_do() {
        let mut maker = super::CardMaker::new();
        let mut cards = maker.line("WARN : Lua          f:0 at Lua((MOD:Vanilla Vehicles Animated)).Supra_functions_fix.lua> require(\"Vehicles/Supra_functions\") failed.");
        cards.extend(maker.flush());
        assert_eq!(cards.len(), 1);
        let card = &cards[0];
        assert_eq!((card.code.as_str(), card.mod_name.as_deref()), ("mod_named_warning", Some("Vanilla Vehicles Animated")));
        assert_eq!(card.params.get("variant").map(String::as_str), Some("require"));
        assert_eq!(card.params.get("file").map(String::as_str), Some("Vehicles/Supra_functions"));
        assert!(card.recommended_action.starts_with("Nothing to do unless"));
    }
}
