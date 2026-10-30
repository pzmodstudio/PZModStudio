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
                        for card in cards.line(&trimmed) {
                            let _ = app.emit("sandbox-error-card", card);
                        }
                        line_buf.clear();
                    }
                    // Nothing more came: an error's card waiting for its exception line goes out.
                    if !read_any {
                        if let Some(card) = cards.flush() {
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
}

impl CardMaker {
    pub fn new() -> Self {
        CardMaker { blame: ModBlame::default(), already_reported: Default::default(), counter: 1, pending: None, pending_has_detail: false, in_error: false, block_mods: Vec::new() }
    }

    /// A new game session (console.txt started over): what it blamed and reported is forgotten.
    pub fn new_session(&mut self) {
        self.blame = ModBlame::default();
        self.already_reported.clear();
        self.pending = None;
        self.in_error = false;
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
        Some(card)
    }

    fn hold(&mut self, card: TranslatedErrorPayload) {
        self.pending = Some(card);
        self.pending_has_detail = false;
        self.block_mods.clear();
    }

    /// The cards this line completes (usually none, at most two).
    pub fn line(&mut self, line: &str) -> Vec<TranslatedErrorPayload> {
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
        }
        let is_error_level = level && trimmed.starts_with("ERROR");
        let culprit = self.blame.of(line);
        if let Some(mut card) = translate_log_error(line, self.counter) {
            self.counter += 1;
            card.mod_name = culprit.clone();
            if !level && self.pending.is_some() {
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
                let key = format!("{}|{}", name, trimmed.chars().take(80).collect::<String>());
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
}

/// How many lines a mod's name carries forward. The frame and its error are next to each other;
/// a wider window would start blaming whoever happened to be mentioned earlier.
const BLAME_WINDOW: usize = 3;

impl ModBlame {
    /// Reads `mod "X" overrides <path>` lines as they go by, and remembers a mod named in a
    /// stack frame for the few lines that follow it.
    pub fn observe(&mut self, line: &str) {
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
            }
        } else if let Some(id) = Self::item_mod(line) {
            // `CreateItem() cannot create item: ... modID = "X"` comes right before the exception
            // the game throws loading that item from a save.
            self.recent = Some((id, 0));
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

    /// The mod this line is about, if the game said.
    pub fn of(&self, line: &str) -> Option<String> {
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
        // Failing both: the mod named a line or two above, which for a Java exception is the
        // Lua frame that threw it.
        self.recent.as_ref().map(|(name, _)| name.clone())
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

/// The last count, kept while console.txt is unchanged: Mod Health asks on every refresh, and a
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

/// A line worth showing even though it matches no known pattern, because the game named a mod.
fn blame_card(line: &str, mod_name: &str, counter: usize) -> TranslatedErrorPayload {
    let severity = Severity::of_line(line);
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
        params: crate::catalog::issue_params(&[("mod", mod_name)]),
        raw_error: line.to_string(),
        source_file,
        line_number: None,
        title: format!("{} reported a problem", mod_name),
        explanation: if detail.is_empty() { line.to_string() } else { detail },
        recommended_action: match severity {
            Severity::Exception => format!(
                "The game named {} on this line. If something is misbehaving in game, switching that mod off is the quickest way to tell whether it is the cause.",
                mod_name
            ),
            Severity::Warning => format!(
                "The game named {} here and carried on. Nothing to do unless something in that mod misbehaves in game; then this line is where to look.",
                mod_name
            ),
        },
        polyfill_rule_id_suggestion: None,
        mod_name: Some(mod_name.to_string()),
        severity,
    }
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

/// Translates raw Java stacktraces & Lua errors into actionable cards.
pub fn translate_log_error(line: &str, counter: usize) -> Option<TranslatedErrorPayload> {
    let mut card = translate_log_error_rule(line, counter)?;
    if card.code != "ui_draw_reload" {
        card.severity = Severity::of_line(line);
    }
    Some(card)
}

fn translate_log_error_rule(line: &str, counter: usize) -> Option<TranslatedErrorPayload> {
    // A card says something went wrong. The rules below match on names a mod puts in the log --
    // "FancyHandwork", "SaucedCarts", "dSAG" -- and the game writes those names when the mod
    // merely loads, so without this the Monitor reported `LOG : Mod f:0> loading FancyHandwork`
    // as an uncaught error on tick. Nothing but a line the game itself marked as trouble.
    let is_trouble = line.starts_with("ERROR")
        || line.starts_with("WARN")
        || line.contains("Exception")
        || line.contains("stack trace")
        || line.contains("Error:");
    if !is_trouble {
        return None;
    }
    if line.contains("isLocalPlayer()") || line.contains("ToggleDoorActual") || line.contains("ToggleWindow") {
        Some(TranslatedErrorPayload {
            called_from: Vec::new(),
            id: format!("err_{}", counter),
            code: "door_npc".to_string(),
            params: BTreeMap::new(),
            raw_error: line.to_string(),
            source_file: Some("zombie/iso/objects/IsoDoor.java".to_string()),
            line_number: None,
            title: "The game's own door code met a missing player (B42)".to_string(),
            explanation: "A zombie or an NPC opened, thumped or broke a door or window, and the game's Java (IsoDoor.ToggleDoorActual) asked a player that does not exist whether it is the local one. It is the game's code, reached without any Lua: NPC mods (Project A-Life, Bandits) make it happen more often. The game catches it and carries on.".to_string(),
            recommended_action: "Nothing to patch: no Lua runs on that path, so no package or shim can reach it. Ignore it unless it floods the log; then the mod that moves the NPC through the door is the one to report it to.".to_string(),
            mod_name: None,
            severity: Severity::Exception,
            polyfill_rule_id_suggestion: None,
        })
    } else if line.contains("BWORoomPrograms") || (line.contains("restaurant") && line.contains("getName")) {
        Some(TranslatedErrorPayload {
            called_from: Vec::new(),
            id: format!("err_{}", counter),
            code: "bandits_room".to_string(),
            params: BTreeMap::new(),
            raw_error: line.to_string(),
            source_file: Some("media/lua/shared/BWORoomPrograms.lua".to_string()),
            line_number: None,
            title: "Bandits Week One Dining Room Safety (B42)".to_string(),
            explanation: "Bandits Week One room program attempted to query getName() on an outdoor/unzoned tile where getRoom() is nil, generating error floods on each AI tick.".to_string(),
            recommended_action: "Apply Polyfill Rule: B42_BANDITS_ROOM_SAFETY".to_string(),
            mod_name: None,
            severity: Severity::Exception,
            polyfill_rule_id_suggestion: Some("B42_BANDITS_ROOM_SAFETY".to_string()),
        })
    } else if line.contains("dSAG") || line.contains("onNewDay") || line.contains("ReportWindow") {
        Some(TranslatedErrorPayload {
            called_from: Vec::new(),
            id: format!("err_{}", counter),
            code: "daily_report".to_string(),
            params: BTreeMap::new(),
            raw_error: line.to_string(),
            source_file: Some("media/lua/client/dSAG/dSAG_Window.lua".to_string()),
            line_number: None,
            title: "Daily Report Journal Callback Protection".to_string(),
            explanation: "Daily Report Journal rollover or player death event executed callbacks on an uninitialized UI window.".to_string(),
            recommended_action: "Apply Polyfill Rule: B42_DAILY_REPORT_SAFETY".to_string(),
            mod_name: None,
            severity: Severity::Exception,
            polyfill_rule_id_suggestion: Some("B42_DAILY_REPORT_SAFETY".to_string()),
        })
    } else if line.contains("UnknownFormatConversionException") || line.contains("Translator.reportMissingArgumentsFromPastAbuse") {
        // Every way Java refuses a bare % in a text: a sign it cannot read ("Conversion = ')'"),
        // one that reads as a placeholder with nothing to fill it ("Format specifier '% A'"), or
        // one with flags that do not fit ("FormatFlagsConversionMismatchException").
        Some(TranslatedErrorPayload {
            called_from: Vec::new(),
            id: format!("err_{}", counter),
            code: "translator_format".to_string(),
            params: BTreeMap::new(),
            raw_error: line.to_string(),
            source_file: Some("zombie/core/Translator.java".to_string()),
            line_number: None,
            title: "Translator Format Exception (% character)".to_string(),
            explanation: "A mod called Translator.getText() with an unescaped % or . character, causing Java String formatting to collapse.".to_string(),
            recommended_action: "Apply Polyfill Rule: SANITIZE_TRANSLATOR_FORMAT".to_string(),
            mod_name: None,
            severity: Severity::Exception,
            polyfill_rule_id_suggestion: Some("SANITIZE_TRANSLATOR_FORMAT".to_string()),
        })
        .map(|card| {
            // A text in a mod's translation file with a bare % that Java formats (a sandbox
            // option's name, `Formatting "Sandbox_X_Y"`, or a mod's options panel, from
            // MainOptions.addModOptionsPanel). The game shows it unformatted and goes on; the
            // shim, being Lua, never sees it, so the card says where the fix is instead of
            // pointing at the Merger.
            TranslatedErrorPayload {
                called_from: Vec::new(),
                code: "translator_text".to_string(),
                source_file: None,
                title: "A mod's text has a bare % sign".to_string(),
                explanation: "A text in a mod's translation file (a sandbox option's name or tooltip, usually) has a % that is not written %%, so Java's text formatting cannot read it. The game shows the text as it is and carries on: nothing else breaks.".to_string(),
                recommended_action: "Nothing to do to play. The fix is in the mod's translation file (% written as %%); Save report names the mod, the file and the line.".to_string(),
                severity: Severity::Warning,
                polyfill_rule_id_suggestion: None,
                ..card
            }
        })
    } else if let Some(method) = ui_method_of_nil(line) {
        // `attempted index: prerender of non-table: null` under a window's render/prerender:
        // `ISPanelJoypad.prerender(self)` with ISPanelJoypad nil, which only happens while the
        // game loads its Lua again (a save loading, back to the menu) and a window that
        // survived the reload draws a couple of frames before its base class is back.
        Some(TranslatedErrorPayload {
            called_from: Vec::new(),
            id: format!("err_{}", counter),
            code: "ui_draw_reload".to_string(),
            params: crate::catalog::issue_params(&[("method", method)]),
            raw_error: line.to_string(),
            source_file: None,
            line_number: None,
            title: "A window drew while the game was reloading its Lua".to_string(),
            explanation: format!("A window of this mod ran its {} while the game was loading its Lua again (a save loading, or back to the menu), a couple of frames before the class it is built on existed. Once the reload is done the window is fine.", method),
            recommended_action: "Nothing to do: it is timing during the reload, not a fault in play. The mod's author can guard that call; for you nothing changes.".to_string(),
            mod_name: None,
            severity: Severity::Warning,
            polyfill_rule_id_suggestion: None,
        })
    } else if line.contains("attempted index of non-table") || line.contains("attempted index: getName of non-table") {
        Some(TranslatedErrorPayload {
            called_from: Vec::new(),
            id: format!("err_{}", counter),
            code: "nil_table".to_string(),
            params: BTreeMap::new(),
            raw_error: line.to_string(),
            source_file: None,
            line_number: None,
            title: "Uninitialized Global Table Access".to_string(),
            explanation: "Mod code attempted to access properties of a nil/uninitialized table before PZ Build 42 instantiated it.".to_string(),
            recommended_action: "Apply Polyfill Rule: SAFE_GLOBAL_TABLE_ACCESS".to_string(),
            mod_name: None,
            severity: Severity::Exception,
            polyfill_rule_id_suggestion: Some("SAFE_GLOBAL_TABLE_ACCESS".to_string()),
        })
    } else if line.contains("NullPointerException") && line.contains("InventoryItem.load(") && line.contains("is null") {
        // Loading a save: an item in it (often a weapon part) could not be created, the line
        // before says which and from which mod (`cannot create item ... removed = true`). The
        // game skips it and goes on. Java, while reading the save: the shim never sees it.
        Some(TranslatedErrorPayload {
            called_from: Vec::new(),
            id: format!("err_{}", counter),
            code: "save_item_missing".to_string(),
            params: BTreeMap::new(),
            raw_error: line.to_string(),
            source_file: None,
            line_number: None,
            title: "An item in the save no longer exists".to_string(),
            explanation: "Loading the save, the game found an item (often a weapon part or something inside a container) that it cannot create any more: the mod that made it is off, or no longer defines it, or the save marked it removed when it was played without that mod. The game skips the item and carries on; that item is gone from the save.".to_string(),
            recommended_action: "Nothing to do to keep playing. If the item matters, switch the mod named here on before loading the save. The line just above in the log (\"cannot create item\") names the item.".to_string(),
            mod_name: None,
            severity: Severity::Warning,
            polyfill_rule_id_suggestion: None,
        })
    } else if line.contains("NullPointerException")
        && (line.contains("ConnectToServerState") || line.contains("GameClient.connection") || line.contains("UdpConnection"))
    {
        // Joining a server: the connection was gone when the game wanted to use it (it drops
        // while a long Workshop download runs). The game's, not a mod's: no polyfill or merge
        // reaches it; joining again does, with the mods already downloaded.
        Some(TranslatedErrorPayload {
            called_from: Vec::new(),
            id: format!("err_{}", counter),
            code: "connection_lost".to_string(),
            params: BTreeMap::new(),
            raw_error: line.to_string(),
            source_file: None,
            line_number: None,
            title: "The connection to the server was lost while joining".to_string(),
            explanation: "While joining, the connection to the server closed (it often does while the game downloads the server's mods), and the game then failed on it and stayed on a blank screen. It is the game's connection, not a mod.".to_string(),
            recommended_action: "Close the game and join again: what it downloaded is kept, so this time it goes straight in. A warning just before (\"required mod ... not found\") is a mod the game only sees after it restarts.".to_string(),
            mod_name: None,
            severity: Severity::Exception,
            polyfill_rule_id_suggestion: None,
        })
    } else if line.contains("NullPointerException") {
        Some(TranslatedErrorPayload {
            called_from: Vec::new(),
            id: format!("err_{}", counter),
            code: "java_npe".to_string(),
            params: BTreeMap::new(),
            raw_error: line.to_string(),
            source_file: None,
            line_number: None,
            title: "Java Interop Null Pointer Exception".to_string(),
            explanation: "A Java method returned null or received a null parameter from a legacy Lua mod call.".to_string(),
            recommended_action: "Apply Universal Master Polyfills to handle null Java interop arguments safely.".to_string(),
            mod_name: None,
            severity: Severity::Exception,
            polyfill_rule_id_suggestion: Some("B42_OBJECT_INTERACT_SAFETY".to_string()),
        })
    } else if line.contains("KahluaThreadException") || line.contains("se.krka.kahlua.vm.KahluaException") {
        Some(TranslatedErrorPayload {
            called_from: Vec::new(),
            id: format!("err_{}", counter),
            code: "kahlua".to_string(),
            params: BTreeMap::new(),
            raw_error: line.to_string(),
            source_file: None,
            line_number: None,
            title: "Lua Runtime Execution Crash (Kahlua)".to_string(),
            explanation: "An unhandled Lua runtime exception was thrown inside the PZ Kahlua VM.".to_string(),
            recommended_action: "Apply Universal Master Polyfills to protect game loops and callframe stacktraces.".to_string(),
            mod_name: None,
            severity: Severity::Exception,
            polyfill_rule_id_suggestion: Some("SAFE_GLOBAL_TABLE_ACCESS".to_string()),
        })
    } else if line.contains("IndexOutOfBoundsException") || line.contains("Index 0 out of bounds") {
        Some(TranslatedErrorPayload {
            called_from: Vec::new(),
            id: format!("err_{}", counter),
            code: "index_bounds".to_string(),
            params: BTreeMap::new(),
            raw_error: line.to_string(),
            source_file: Some("java/util/ArrayList.java".to_string()),
            line_number: None,
            title: "Multiplayer Java ArrayList Out of Bounds (B42)".to_string(),
            explanation: "A mod accessed IsoPlayer.getPlayers() or getOnlinePlayers() index 0 while the Java player collection was empty during a render/tick loop in Multiplayer.".to_string(),
            recommended_action: "Apply Universal Master Polyfills (Safe ArrayList Proxy & getOnlinePlayers shield).".to_string(),
            mod_name: None,
            severity: Severity::Exception,
            polyfill_rule_id_suggestion: Some("B42_DAILY_REPORT_SAFETY".to_string()),
        })
    } else if line.contains("FancyHandwork") || line.contains("fancyMP") || line.contains("aFancyHandwork") {
        Some(TranslatedErrorPayload {
            called_from: Vec::new(),
            id: format!("err_{}", counter),
            code: "fancy_handwork".to_string(),
            params: BTreeMap::new(),
            raw_error: line.to_string(),
            source_file: Some("media/lua/client/aFancyHandwork.lua".to_string()),
            line_number: None,
            title: "Fancy Handwork Multiplayer Hand Sync Safety".to_string(),
            explanation: "Fancy Handwork fancyMP loop threw an uncaught error when evaluating local/remote player hand objects on tick.".to_string(),
            recommended_action: "Apply Universal Master Polyfills to guard FancyHandwork callbacks safely.".to_string(),
            mod_name: None,
            severity: Severity::Exception,
            polyfill_rule_id_suggestion: Some("SAFE_GLOBAL_TABLE_ACCESS".to_string()),
        })
    } else if line.contains("SaucedCarts") || line.contains("Pushable Carts") {
        Some(TranslatedErrorPayload {
            called_from: Vec::new(),
            id: format!("err_{}", counter),
            code: "sauced_carts".to_string(),
            params: BTreeMap::new(),
            raw_error: line.to_string(),
            source_file: Some("media/lua/client/SaucedCarts.lua".to_string()),
            line_number: None,
            title: "SaucedCarts Pushable Carts Multiplayer Sync".to_string(),
            explanation: "Pushable Carts vehicle/entity interaction threw a null pointer exception during multiplayer state updates.".to_string(),
            recommended_action: "Apply Universal Master Polyfills to guard SaucedCarts interop.".to_string(),
            mod_name: None,
            severity: Severity::Exception,
            polyfill_rule_id_suggestion: Some("SAFE_GLOBAL_TABLE_ACCESS".to_string()),
        })
    } else if line.contains("duplicate recipe") || line.contains("duplicate item") {
        Some(TranslatedErrorPayload {
            called_from: Vec::new(),
            id: format!("err_{}", counter),
            code: "duplicate_id".to_string(),
            params: BTreeMap::new(),
            raw_error: line.to_string(),
            source_file: None,
            line_number: None,
            title: "Duplicate Item/Recipe ID Collision".to_string(),
            explanation: "Multiple active mods registered the same script ID or recipe name, causing overwrite conflicts.".to_string(),
            recommended_action: "Use Merger Module to merge script definitions into Master Patch.".to_string(),
            mod_name: None,
            severity: Severity::Exception,
            polyfill_rule_id_suggestion: None,
        })
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
        assert_eq!(card.explanation, "require(\"NCUtils\") failed");
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
    fn a_real_error_line_still_makes_its_card() {
        let card = translate_log_error(
            "ERROR: General      f:304> 20)).FHhandleKeybinds> Exception thrown aFancyHandwork.lua:126",
            1,
        );
        assert!(card.is_some());
        assert!(card.unwrap().title.contains("Fancy Handwork"));
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

        let threw = translate_log_error(
            "ERROR: General      f:304> 20)).FHhandleKeybinds> Exception thrown aFancyHandwork.lua:126",
            2,
        )
        .expect("a card");
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
        assert!(card.explanation.starts_with("java.lang.RuntimeException: attempted index: getUnequippedWeight"), "{}", card.explanation);
        assert!(card.raw_error.starts_with("ERROR:"));
        assert!(maker.flush().is_none());

        // The same error again (every tick): no new card, and no card for its exception either.
        let again = run(&mut maker, &BLOCK);
        let again_end = maker.line("LOG  : General      f:29307> done");
        assert!(again.is_empty() && again_end.is_empty());
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
        assert!(link.explanation.starts_with("java.lang.RuntimeException"));
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
