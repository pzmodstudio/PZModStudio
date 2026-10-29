//! The Studio's own log, and the diagnostics a player can paste into a bug report.
//!
//! - `<Zomboid>/PZModStudio_Logs/studio.log`: what went wrong in the Studio. Its panics (with
//!   where they happened) and every error the window showed. It rotates at 1 MB, keeping three,
//!   so it never grows for ever.
//! - A panic is also sent to the window (`studio-panic`), so a crash of one background job is
//!   seen, not only logged.
//! - `report`: versions, folders, the game build, the profile, the Studio Link, Zombie Buddy,
//!   Steam's state and the end of the log, as plain text. The Windows user name is replaced with
//!   `<user>` wherever it is a path component (folders), and so is the PC's name.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

pub const PANIC_EVENT: &str = "studio-panic";
const LOG_DIR: &str = "PZModStudio_Logs";
const LOG_FILE: &str = "studio.log";
const LOG_MAX_BYTES: u64 = 1024 * 1024;
const LOG_KEPT: usize = 3;

static WRITE_LOCK: Mutex<()> = Mutex::new(());
/// Set by tests; else the Zomboid folder the Studio uses.
static LOG_DIR_OVERRIDE: Mutex<Option<PathBuf>> = Mutex::new(None);

fn log_dir() -> Option<PathBuf> {
    if let Some(dir) = LOG_DIR_OVERRIDE.lock().ok().and_then(|g| g.clone()) {
        return Some(dir);
    }
    let chosen = crate::vfs::studio_paths().user_zomboid_dir;
    let z = crate::load_order::mod_info::get_existing_user_zomboid_dirs(&chosen).into_iter().next()?;
    Some(z.join(LOG_DIR))
}

pub fn log_path() -> Option<PathBuf> {
    log_dir().map(|d| d.join(LOG_FILE))
}

/// `studio.log` -> `studio.1.log` -> `studio.2.log`; the oldest goes.
fn rotate(dir: &Path) {
    let name = |n: usize| if n == 0 { dir.join(LOG_FILE) } else { dir.join(format!("studio.{}.log", n)) };
    let _ = std::fs::remove_file(name(LOG_KEPT - 1));
    for n in (0..LOG_KEPT - 1).rev() {
        let _ = std::fs::rename(name(n), name(n + 1));
    }
}

/// One line in the Studio's log: `<local time> <LEVEL> <text>`. Never fails the caller.
pub fn log(level: &str, text: &str) {
    let Some(dir) = log_dir() else { return };
    let _guard = WRITE_LOCK.lock();
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    let path = dir.join(LOG_FILE);
    if std::fs::metadata(&path).is_ok_and(|m| m.len() > LOG_MAX_BYTES) {
        rotate(&dir);
    }
    let line = format!("{} {:<5} {}\n", chrono::Local::now().format("%Y-%m-%d %H:%M:%S"), level, text.replace(['\r', '\n'], " | "));
    if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&path) {
        let _ = f.write_all(line.as_bytes());
    }
}

/// An error of a background job: into the Studio's log (and the console, for development).
pub fn error(text: String) {
    eprintln!("{}", text);
    log("ERROR", &text);
}

static WINDOW: OnceLock<Box<dyn Fn(String) + Send + Sync>> = OnceLock::new();

/// From now on a panic anywhere in the Studio is logged with where it happened, and told to the
/// window through `notify`.
pub fn install_panic_hook(notify: impl Fn(String) + Send + Sync + 'static) {
    let _ = WINDOW.set(Box::new(notify));
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let what = info
            .payload()
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| info.payload().downcast_ref::<String>().cloned())
            .unwrap_or_else(|| "unknown".to_string());
        let place = info.location().map(|l| format!("{}:{}", l.file(), l.line())).unwrap_or_default();
        let thread = std::thread::current().name().unwrap_or("background").to_string();
        let text = format!("PANIC in {} at {}: {}", thread, place, what);
        log("PANIC", &text);
        if let Some(notify) = WINDOW.get() {
            notify(mask(&text));
        }
        previous(info);
    }));
}

/// The text with the Windows user name and the PC name taken out where they are a part of a
/// path (`C:\Users\<user>\...`) or stand alone, never inside another word.
pub fn mask(text: &str) -> String {
    let mut out = text.to_string();
    for (var, placeholder) in [("USERNAME", "<user>"), ("COMPUTERNAME", "<pc>")] {
        if let Ok(name) = std::env::var(var) {
            out = mask_word(&out, &name, placeholder);
        }
    }
    // The home folder's own name, in case it is not the user name.
    if let Some(home) = dirs_next::home_dir().and_then(|h| h.file_name().map(|n| n.to_string_lossy().to_string())) {
        out = mask_word(&out, &home, "<user>");
    }
    out
}

/// Replaces `word` (case-insensitively) where it is not part of a longer name: bordered by a
/// path separator, the start or end, or something that is not a letter, digit, `_` or `-`.
pub fn mask_word(text: &str, word: &str, placeholder: &str) -> String {
    if word.len() < 2 {
        return text.to_string();
    }
    let lower = text.to_lowercase();
    let w = word.to_lowercase();
    let is_name = |c: char| c.is_alphanumeric() || c == '_' || c == '-' || c == '.';
    let mut out = String::with_capacity(text.len());
    let mut i = 0;
    while let Some(found) = lower[i..].find(&w) {
        let start = i + found;
        let end = start + w.len();
        let before_ok = text[..start].chars().next_back().is_none_or(|c| !is_name(c));
        let after_ok = text[end..].chars().next().is_none_or(|c| !is_name(c));
        out.push_str(&text[i..start]);
        if before_ok && after_ok {
            out.push_str(placeholder);
        } else {
            out.push_str(&text[start..end]);
        }
        i = end;
    }
    out.push_str(&text[i..]);
    out
}

fn windows_version() -> String {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        if let Ok(out) = std::process::Command::new("cmd").args(["/c", "ver"]).creation_flags(0x08000000).output() {
            let v = String::from_utf8_lossy(&out.stdout).trim().to_string();
            if !v.is_empty() {
                return v;
            }
        }
    }
    format!("{} {}", std::env::consts::OS, std::env::consts::ARCH)
}

fn last_lines(path: &Path, n: usize) -> Vec<String> {
    let Ok(text) = std::fs::read_to_string(path) else { return Vec::new() };
    let lines: Vec<&str> = text.lines().collect();
    lines[lines.len().saturating_sub(n)..].iter().map(|l| l.to_string()).collect()
}

fn yes(b: bool) -> &'static str {
    if b { "yes" } else { "no" }
}

/// Everything a bug report needs about this PC's Studio, as plain text, with the user name and
/// the PC's name masked.
pub fn report(user_zomboid_dir: &Path, pz_install_dir: &str, workshop_dir: &str) -> String {
    let mut r = String::new();
    let mut line = |k: &str, v: String| r.push_str(&format!("{:<22} {}\n", k, v));
    line("PZ Mod Studio", env!("CARGO_PKG_VERSION").to_string());
    line("Windows", windows_version());
    line("Zomboid folder", user_zomboid_dir.display().to_string());
    line("Game folder", if pz_install_dir.is_empty() { "(not found)".into() } else { pz_install_dir.to_string() });
    line("Workshop folder", if workshop_dir.is_empty() { "(not found)".into() } else { workshop_dir.to_string() });
    line(
        "Game build",
        crate::load_order::mod_info::read_installed_game_version(&user_zomboid_dir.to_string_lossy()).map(|v| v.to_string()).unwrap_or_else(|| "(unknown)".into()),
    );

    let active = crate::load_order::ini_parser::try_parse_default_txt(&user_zomboid_dir.join("mods").join("default.txt"));
    line("Mods on (default.txt)", active.as_ref().map(|a| a.len().to_string()).unwrap_or_else(|| "(cannot be read)".into()));
    let installed_local = std::fs::read_dir(user_zomboid_dir.join("mods")).map(|d| d.flatten().filter(|e| e.path().is_dir()).count()).unwrap_or(0);
    let workshop_items = if workshop_dir.is_empty() { 0 } else { std::fs::read_dir(workshop_dir).map(|d| d.count()).unwrap_or(0) };
    line("Installed", format!("{} Workshop items, {} local mod folders", workshop_items, installed_local));

    // The active profile, from the Link file the Studio writes.
    let link = std::fs::read_to_string(user_zomboid_dir.join("Lua").join(crate::studio_link::LINK_FILE)).unwrap_or_default();
    let profile = link.lines().find_map(|l| l.strip_prefix("profile=")).unwrap_or("(none)").to_string();
    let members = link.lines().filter(|l| l.starts_with("member=")).count();
    line("Active profile", format!("{} ({} mods)", if profile.is_empty() { "(unnamed)".into() } else { profile }, members));
    let link_info = std::fs::read_to_string(user_zomboid_dir.join("mods").join(crate::studio_link::LINK_MOD_ID).join("42").join("mod.info")).unwrap_or_default();
    let link_version = link_info.lines().find_map(|l| l.strip_prefix("modversion=")).unwrap_or("(not installed locally)").to_string();
    line("Studio Link", link_version);
    let on = |id: &str| active.as_ref().is_some_and(|a| a.iter().any(|m| m == id));
    line("Studio Link on", yes(on(crate::studio_link::LINK_MOD_ID)).into());
    let game = std::fs::read_to_string(user_zomboid_dir.join("Lua").join(crate::studio_link::GAME_FILE)).unwrap_or_default();
    line("Game said", game.lines().filter(|l| l.starts_with("state=") || l.starts_with("link=") || l.starts_with("save=")).collect::<Vec<_>>().join(", "));

    let launcher = std::fs::read_to_string(Path::new(pz_install_dir).join("ProjectZomboid64.json")).unwrap_or_default();
    line("Zombie Buddy", format!("on in the list: {}; launch line: {}", yes(on("ZombieBuddy")), yes(launcher.contains("zbNative"))));
    line("Launch change waiting", yes(user_zomboid_dir.join(crate::studio_link::AGENTS_PENDING_FILE).is_file()).into());
    line("Game running", yes(crate::sandbox::find_running_pz_pid().is_some()).into());

    let saves = crate::save_backup::list_saves(user_zomboid_dir);
    let space = crate::save_backup::disk_space(user_zomboid_dir).map(|(f, t)| format!("{:.1} GB free of {:.1} GB", f as f64 / 1e9, t as f64 / 1e9)).unwrap_or_else(|| "(unknown)".into());
    line("Saves", format!("{}; disk: {}", saves.len(), space));
    line("Steam", crate::workshop::polite::state());

    r.push_str("\nStudio log (last 40 lines):\n");
    match log_path() {
        Some(p) if p.is_file() => {
            for l in last_lines(&p, 40) {
                r.push_str("  ");
                r.push_str(&l);
                r.push('\n');
            }
        }
        _ => r.push_str("  (empty)\n"),
    }
    mask(&r)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_user_name_is_masked_only_where_it_is_a_name() {
        let t = "C:\\Users\\javie\\Zomboid and D:/Users/Javie/x, javier is someone else, javie_mod stays, user javie.";
        assert_eq!(
            mask_word(t, "javie", "<user>"),
            "C:\\Users\\<user>\\Zomboid and D:/Users/<user>/x, javier is someone else, javie_mod stays, user javie."
        );
        assert_eq!(mask_word("javie", "javie", "<user>"), "<user>");
        assert_eq!(mask_word("abc", "a", "<user>"), "abc", "one letter is never masked");
    }

    #[test]
    fn the_log_rotates_and_keeps_three() {
        let dir = std::env::temp_dir().join(format!("pzms-log-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        *LOG_DIR_OVERRIDE.lock().unwrap() = Some(dir.clone());
        log("INFO", "first\nsecond line");
        let text = std::fs::read_to_string(dir.join(LOG_FILE)).unwrap();
        assert!(text.contains("INFO  first | second line"));
        for _ in 0..4 {
            std::fs::write(dir.join(LOG_FILE), vec![b'x'; (LOG_MAX_BYTES + 1) as usize]).unwrap();
            log("ERROR", "after rotation");
        }
        let files: Vec<String> = std::fs::read_dir(&dir).unwrap().flatten().map(|e| e.file_name().to_string_lossy().to_string()).collect();
        assert_eq!(files.len(), LOG_KEPT, "{:?}", files);
        assert!(std::fs::read_to_string(dir.join(LOG_FILE)).unwrap().contains("after rotation"));
        *LOG_DIR_OVERRIDE.lock().unwrap() = None;
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[cfg(test)]
mod live {
    /// `cargo test -- --ignored live_report --nocapture`: the report on this PC (masked).
    #[test]
    #[ignore]
    fn live_report() {
        let p = crate::vfs::studio_paths();
        println!("{}", super::report(std::path::Path::new(&p.user_zomboid_dir), &p.pz_install_dir, &p.workshop_dir));
    }
}
