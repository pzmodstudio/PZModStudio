//! For the people who make the mods: check a whole mod before the game does, and open the
//! file that is wrong in their editor.
//!
//! Every `.lua` under the mod goes through the same parser the Merger uses (full_moon, on a
//! big stack), and every `media/scripts/*.txt` through a brace check — the game's script reader
//! stops at the first unbalanced block and the whole file is silently lost. What comes back is
//! one row per problem: file, line, column, what the parser said.

use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FileProblem {
    /// From the mod's root, with forward slashes.
    pub path: String,
    pub line: Option<u32>,
    pub col: Option<u32>,
    pub message: String,
    /// `lua` or `script`.
    pub kind: String,
    /// Which problem it is, for the app to say in the player's language: `script_extra_close`,
    /// `script_unclosed`, `not_utf8`. Empty for the parser's own words, which stay as written.
    #[serde(default)]
    pub code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ModValidation {
    pub root: String,
    pub lua_files: usize,
    pub script_files: usize,
    pub problems: Vec<FileProblem>,
}

/// The parser's message split into one problem per error, each with its line and column.
/// full_moon writes "error occurred while creating ast: unexpected token `end`. (starting from
/// line 4, character 1 and ending on line 4, character 4)\nadditional information: ..." and joins
/// several with "; ".
pub fn lua_problems(path: &str, message: &str) -> Vec<FileProblem> {
    let message = message.trim_start_matches("Lua AST Syntax Error: ");
    let mut out = Vec::new();
    for part in message.split("error occurred while creating ast: ") {
        let part = part.trim().trim_end_matches(';').trim();
        if part.is_empty() {
            continue;
        }
        let (text, rest) = part.split_once(" (starting from line ").unwrap_or((part, ""));
        let line = rest.split(',').next().and_then(|n| n.trim().parse::<u32>().ok());
        let col = rest.split_once("character ").and_then(|(_, c)| c.split(|ch: char| !ch.is_ascii_digit()).next()?.parse::<u32>().ok());
        let extra = part.split_once("additional information: ").map(|(_, e)| e.trim_end_matches(';').trim());
        let mut text = text.trim_end_matches('.').to_string();
        if let Some(e) = extra {
            text = format!("{} ({})", text, e);
        }
        out.push(FileProblem { path: path.to_string(), line, col, message: text, kind: "lua".into(), code: String::new() });
    }
    if out.is_empty() {
        out.push(FileProblem { path: path.to_string(), line: None, col: None, message: message.to_string(), kind: "lua".into(), code: String::new() });
    }
    out
}

/// A script file's braces, block by block: an extra `}` and an unclosed `{` are each a problem
/// with the line it happened on. Comments (`/* */` and `//`) and quotes are skipped the way the
/// game skips them.
pub fn script_problems(path: &str, text: &str) -> Vec<FileProblem> {
    let mut out = Vec::new();
    let mut open: Vec<u32> = Vec::new();
    let mut line: u32 = 1;
    let mut chars = text.chars().peekable();
    let mut in_block_comment = false;
    let mut in_line_comment = false;
    while let Some(c) = chars.next() {
        if c == '\n' {
            line += 1;
            in_line_comment = false;
            continue;
        }
        if in_block_comment {
            if c == '*' && chars.peek() == Some(&'/') {
                chars.next();
                in_block_comment = false;
            }
            continue;
        }
        if in_line_comment {
            continue;
        }
        match c {
            '/' if chars.peek() == Some(&'*') => {
                chars.next();
                in_block_comment = true;
            }
            '/' if chars.peek() == Some(&'/') => {
                chars.next();
                in_line_comment = true;
            }
            '{' => open.push(line),
            '}' => {
                if open.pop().is_none() {
                    out.push(FileProblem { path: path.to_string(), line: Some(line), col: None, message: "a `}` with no block open; the game's script reader stops here".into(), kind: "script".into(), code: "script_extra_close".into() });
                }
            }
            _ => {}
        }
    }
    for opened in open {
        out.push(FileProblem { path: path.to_string(), line: Some(opened), col: None, message: "a `{` never closed; the rest of the file is lost to the game".into(), kind: "script".into(), code: "script_unclosed".into() });
    }
    out
}

fn relative(root: &Path, file: &Path) -> String {
    file.strip_prefix(root).unwrap_or(file).to_string_lossy().replace('\\', "/")
}

/// Every Lua and script file of the mod, checked.
pub fn validate_mod(root: &Path) -> Result<ModValidation, String> {
    if !root.is_dir() {
        return Err(format!("{} is not a folder.", root.display()));
    }
    let mut v = ModValidation { root: root.to_string_lossy().to_string(), ..Default::default() };
    for entry in walkdir::WalkDir::new(root).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let rel = relative(root, path);
        let lower = rel.to_lowercase();
        if lower.ends_with(".lua") {
            v.lua_files += 1;
            let Ok(text) = std::fs::read_to_string(path) else {
                v.problems.push(FileProblem { path: rel, line: None, col: None, message: "not UTF-8 text; the game reads it, this check cannot".into(), kind: "lua".into(), code: "not_utf8".into() });
                continue;
            };
            let result = crate::diff_engine::lua::validate_lua_syntax(&text);
            if let Some(msg) = result.error_message {
                v.problems.extend(lua_problems(&rel, &msg));
            }
        } else if lower.ends_with(".txt") && (lower.contains("/media/scripts/") || lower.starts_with("media/scripts/")) {
            v.script_files += 1;
            let Ok(text) = std::fs::read_to_string(path) else { continue };
            v.problems.extend(script_problems(&rel, &text));
        }
    }
    v.problems.sort_by(|a, b| a.path.cmp(&b.path).then(a.line.cmp(&b.line)));
    Ok(v)
}

/// Opens a file at a line in VS Code when it is on the PATH (`code --goto file:line:col`), else
/// in whatever Windows opens the file with.
pub fn open_in_editor(file: &Path, line: Option<u32>, col: Option<u32>) -> Result<String, String> {
    if !file.is_file() {
        return Err(format!("{} is not there.", file.display()));
    }
    // A canonical path carries the \\?\ prefix, which editors do not expect.
    let shown = file.display().to_string();
    let shown = shown.strip_prefix(r"\\?\").unwrap_or(&shown).to_string();
    let target = match (line, col) {
        (Some(l), Some(c)) => format!("{}:{}:{}", shown, l, c),
        (Some(l), None) => format!("{}:{}", shown, l),
        _ => shown.clone(),
    };
    // VS Code's own executable, started directly: going through `cmd /C code` would hand the
    // file name (a mod's, so anyone's) to cmd.exe, which runs whatever follows a `&` in it.
    if let Some(code) = find_vscode() {
        if std::process::Command::new(code).args(["--goto", &target]).spawn().is_ok() {
            return Ok("VS Code".to_string());
        }
    }
    open::that(&shown).map_err(|e| format!("Windows could not open that file: {}", e))?;
    Ok("the file's own program".to_string())
}

/// VS Code's executable: next to the `code` launcher on the PATH, or where its installers put it.
fn find_vscode() -> Option<PathBuf> {
    if !cfg!(windows) {
        let path = std::env::var_os("PATH")?;
        return std::env::split_paths(&path).map(|d| d.join("code")).find(|p| p.is_file());
    }
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path) {
            if dir.join("code.cmd").is_file() {
                if let Some(parent) = dir.parent() {
                    candidates.push(parent.join("Code.exe"));
                }
            }
        }
    }
    for (var, sub) in [
        ("LOCALAPPDATA", r"Programs\Microsoft VS Code\Code.exe"),
        ("ProgramFiles", r"Microsoft VS Code\Code.exe"),
        ("ProgramFiles(x86)", r"Microsoft VS Code\Code.exe"),
    ] {
        if let Some(base) = std::env::var_os(var) {
            candidates.push(PathBuf::from(base).join(sub));
        }
    }
    candidates.into_iter().find(|p| p.is_file())
}

/// A folder is a mod's when it holds a mod.info, at its root or in a build folder.
pub fn looks_like_a_mod(root: &Path) -> bool {
    root.join("mod.info").is_file()
        || std::fs::read_dir(root).ok().into_iter().flatten().flatten().any(|e| e.path().join("mod.info").is_file())
}

pub fn mod_root(p: &str) -> PathBuf {
    PathBuf::from(p)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_parser_message_becomes_one_problem_per_error_with_its_place() {
        let msg = "Lua AST Syntax Error: error occurred while creating ast: unexpected token `return`. (starting from line 3, character 3 and ending on line 3, character 9)\nadditional information: expected a parameter name or `)`; error occurred while creating ast: unexpected token `end`. (starting from line 4, character 1 and ending on line 4, character 4)\nadditional information: unexpected token, this needs to be a statement";
        let p = lua_problems("media/lua/client/x.lua", msg);
        assert_eq!(p.len(), 2, "{:#?}", p);
        assert_eq!((p[0].line, p[0].col), (Some(3), Some(3)));
        assert_eq!(p[0].message, "unexpected token `return` (expected a parameter name or `)`)");
        assert_eq!((p[1].line, p[1].col), (Some(4), Some(1)));
    }

    #[test]
    fn braces_are_checked_outside_comments() {
        let text = "module Base {\n  item Axe { Type = Weapon, } // a } in a comment\n  /* { */\n  recipe X {\n}\n";
        let p = script_problems("media/scripts/x.txt", text);
        assert_eq!(p.len(), 1, "{:#?}", p);
        assert_eq!(p[0].line, Some(1), "the module block is the one left open");
        let extra = script_problems("x.txt", "a { } }");
        assert_eq!(extra[0].line, Some(1));
        assert!(script_problems("x.txt", "module Base { item A { } }").is_empty());
    }

    #[test]
    fn a_mod_folder_is_walked_and_its_problems_named_by_relative_path() {
        let dir = std::env::temp_dir().join(format!("pzms-devtools-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("42/media/lua/client")).unwrap();
        std::fs::create_dir_all(dir.join("42/media/scripts")).unwrap();
        std::fs::write(dir.join("42/mod.info"), "id=X\n").unwrap();
        std::fs::write(dir.join("42/media/lua/client/ok.lua"), "local a = 1\n").unwrap();
        std::fs::write(dir.join("42/media/lua/client/bad.lua"), "function f(\n").unwrap();
        std::fs::write(dir.join("42/media/scripts/items.txt"), "module Base {\n").unwrap();
        let v = validate_mod(&dir).unwrap();
        assert!(looks_like_a_mod(&dir));
        assert_eq!((v.lua_files, v.script_files), (2, 1));
        let paths: Vec<&str> = v.problems.iter().map(|p| p.path.as_str()).collect();
        assert_eq!(paths, vec!["42/media/lua/client/bad.lua", "42/media/scripts/items.txt"], "{:#?}", v.problems);
        let _ = std::fs::remove_dir_all(&dir);
    }
}

// ------------------------------------------------------------------------------------------
// Reload a changed Lua file in the running game, through the bridge.
//
// While the Developer screen watches a mod, Windows says the moment a file under it changes
// (a change notification on the folder, not a look every two seconds). Each changed `.lua` goes
// through the parser first: one that does not parse is not sent, and its line and column are
// shown. The rest go to the bridge's own reload queue (`Zomboid/Lua/pz_dev_reload_queue.txt`),
// which it reads every few frames on the main menu, in game and paused, while
// `pz_dev_watch.txt` says on=1. It calls `reloadLuaFile` for each and writes what happened, and
// when, to `pz_dev_reload.json`. Every stage goes to the window as a `dev-reload` event with its
// time, so the log says "detected -> sent -> applied".
//
// `reloadLuaFile` reroutes the file's event handlers to the new functions itself (an
// `Events.X.Add` during a reload does nothing), so handlers are not doubled; a handler added for
// the first time in the edited file only registers after a full Lua reset.
// ------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChangedFile {
    /// From the mod's root, with forward slashes.
    pub path: String,
    /// On disk, with forward slashes: what `reloadLuaFile` is given.
    pub absolute: String,
    pub modified_ms: u64,
}

fn modified_ms(path: &Path) -> Option<u64> {
    path.metadata().ok()?.modified().ok()?.duration_since(std::time::UNIX_EPOCH).ok().map(|d| d.as_millis() as u64)
}

/// The mod's `.lua` files changed after `since_ms` (unix milliseconds); all of them for 0.
pub fn changed_lua_files(root: &Path, since_ms: u64) -> Vec<ChangedFile> {
    let mut out = Vec::new();
    for entry in walkdir::WalkDir::new(root).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        if !path.is_file() || !path.extension().map(|e| e.eq_ignore_ascii_case("lua")).unwrap_or(false) {
            continue;
        }
        let Some(modified) = modified_ms(path) else { continue };
        if modified > since_ms {
            out.push(ChangedFile {
                path: relative(root, path),
                absolute: path.to_string_lossy().replace('\\', "/"),
                modified_ms: modified,
            });
        }
    }
    out.sort_by(|a, b| a.path.cmp(&b.path));
    out
}

const RELOAD_RESULT_FILE: &str = "pz_dev_reload.json";
const RELOAD_QUEUE_FILE: &str = "pz_dev_reload_queue.txt";
const WATCH_FILE: &str = "pz_dev_watch.txt";
pub const DEV_RELOAD_EVENT: &str = "dev-reload";
/// How long a reload is waited for; the bridge answers within a few frames.
const ANSWER_WAIT_MS: u64 = 6000;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ReloadResult {
    pub id: String,
    /// "ok <file>" or "failed <file>: <error>", one per file.
    pub results: Vec<String>,
    /// When the game applied it (unix ms, the game's clock: the same as this machine's).
    #[serde(default)]
    pub applied_ms: u64,
}

/// One stage of a reload, for the Developer screen's log.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DevReloadEvent {
    /// `syntax` (files not sent), `sent`, `applied`, `no_answer`, `not_running`.
    pub stage: String,
    pub id: String,
    /// From the mod's root.
    pub files: Vec<String>,
    pub results: Vec<String>,
    /// The files that do not parse, with where.
    pub problems: Vec<FileProblem>,
    pub detected_ms: u64,
    pub sent_ms: u64,
    pub applied_ms: u64,
    pub answered_ms: u64,
}

fn now_ms() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
}

/// The queue file the bridge reads: `id=<id>`, then `file=<path>` per file.
pub fn reload_queue_text(id: &str, files: &[String]) -> String {
    let mut out = format!("id={}\n", id);
    for f in files {
        out.push_str(&format!("file={}\n", f.replace(['\r', '\n'], "")));
    }
    out
}

/// What the game wrote back, when it has.
pub fn read_reload_result(user_zomboid_dir: &str) -> Option<ReloadResult> {
    let text = std::fs::read_to_string(Path::new(user_zomboid_dir).join("Lua").join(RELOAD_RESULT_FILE)).ok()?;
    serde_json::from_str::<ReloadResult>(&text).ok()
}

/// The files that parse, and the problems of those that do not.
pub fn check_before_reload(files: &[ChangedFile]) -> (Vec<ChangedFile>, Vec<FileProblem>) {
    let mut good = Vec::new();
    let mut problems = Vec::new();
    for f in files {
        match std::fs::read_to_string(&f.absolute) {
            Ok(text) => match crate::diff_engine::lua::validate_lua_syntax(&text).error_message {
                Some(msg) => problems.extend(lua_problems(&f.path, &msg)),
                None => good.push(f.clone()),
            },
            // Not UTF-8: the game reads it, the parser cannot; sent as it is.
            Err(_) => good.push(f.clone()),
        }
    }
    (good, problems)
}

/// The name `require` knows a file by: its path under `media/lua/<shared|client|server>/`,
/// without `.lua`, lowercased (the game's file lookup ignores case).
fn module_name(path: &str) -> Option<String> {
    let lower = path.replace('\\', "/").to_lowercase();
    let start = ["media/lua/shared/", "media/lua/client/", "media/lua/server/"]
        .iter()
        .find_map(|dir| lower.find(dir).map(|i| i + dir.len()))?;
    Some(lower[start..].trim_end_matches(".lua").to_string())
}

/// The modules a file requires: `require "x"`, `require("x")`, `require 'x'`.
fn required_modules(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(i) = rest.find("require") {
        let after = &rest[i + "require".len()..];
        let before_ok = rest[..i].chars().last().map_or(true, |c| !(c.is_alphanumeric() || c == '_' || c == '.' || c == ':'));
        let arg = after.trim_start().trim_start_matches('(').trim_start();
        if before_ok {
            if let Some(q) = arg.chars().next().filter(|c| *c == '"' || *c == '\'') {
                if let Some(end) = arg[1..].find(q) {
                    out.push(arg[1..1 + end].replace('\\', "/").to_lowercase().trim_end_matches(".lua").to_string());
                }
            }
        }
        rest = after;
    }
    out
}

/// The changed files, then every other file of the mod that requires one of them (and the files
/// that require those), nearest first. `reloadLuaFile` runs a file again but leaves the files
/// that took its module with `local X = require "..."` holding the old one: running them again
/// after it makes them take the new one.
pub fn with_dependents(root: &Path, changed: Vec<ChangedFile>) -> Vec<ChangedFile> {
    let all = changed_lua_files(root, 0);
    let mut users: std::collections::HashMap<String, Vec<usize>> = std::collections::HashMap::new();
    for (i, f) in all.iter().enumerate() {
        let Ok(text) = std::fs::read_to_string(&f.absolute) else { continue };
        for m in required_modules(&text) {
            users.entry(m).or_default().push(i);
        }
    }
    let mut seen: std::collections::HashSet<String> = changed.iter().map(|f| f.absolute.clone()).collect();
    let mut out = changed;
    let mut next = 0;
    while next < out.len() {
        let Some(name) = module_name(&out[next].path) else {
            next += 1;
            continue;
        };
        // `require "Foo/Bar"` names the path from media/lua/<side>/; some mods name only its end.
        let mut found: Vec<usize> = Vec::new();
        for (module, files) in &users {
            if *module == name || name.ends_with(&format!("/{}", module)) {
                found.extend(files);
            }
        }
        found.sort_unstable();
        for i in found {
            if seen.insert(all[i].absolute.clone()) {
                out.push(all[i].clone());
            }
        }
        next += 1;
    }
    out
}

/// Checks these files, hands the good ones to the game and waits for its answer, telling the
/// window at each stage.
pub fn send_reload<R: tauri::Runtime>(app: &tauri::AppHandle<R>, user_zomboid_dir: &str, files: Vec<ChangedFile>, detected_ms: u64) {
    use tauri::Emitter;
    let emit = |e: DevReloadEvent| {
        let _ = app.emit(DEV_RELOAD_EVENT, e);
    };
    let (good, problems) = check_before_reload(&files);
    if !problems.is_empty() {
        emit(DevReloadEvent { stage: "syntax".into(), problems, detected_ms, ..Default::default() });
    }
    if good.is_empty() {
        return;
    }
    let names: Vec<String> = good.iter().map(|f| f.path.clone()).collect();
    if crate::sandbox::find_running_pz_pid().is_none() {
        emit(DevReloadEvent { stage: "not_running".into(), files: names, detected_ms, ..Default::default() });
        return;
    }
    let lua = Path::new(user_zomboid_dir).join("Lua");
    let id = format!("reload_{}", now_ms());
    let _ = std::fs::remove_file(lua.join(RELOAD_RESULT_FILE));
    let absolute: Vec<String> = good.iter().map(|f| f.absolute.clone()).collect();
    if let Err(e) = crate::load_order::ini_parser::atomic_write(lua.join(RELOAD_QUEUE_FILE), reload_queue_text(&id, &absolute)) {
        emit(DevReloadEvent { stage: "no_answer".into(), id, files: names, results: vec![e.to_string()], detected_ms, ..Default::default() });
        return;
    }
    let sent_ms = now_ms();
    emit(DevReloadEvent { stage: "sent".into(), id: id.clone(), files: names.clone(), detected_ms, sent_ms, ..Default::default() });
    while now_ms().saturating_sub(sent_ms) < ANSWER_WAIT_MS {
        if let Some(r) = read_reload_result(user_zomboid_dir).filter(|r| r.id == id) {
            emit(DevReloadEvent { stage: "applied".into(), id, files: names, results: r.results, detected_ms, sent_ms, applied_ms: r.applied_ms, answered_ms: now_ms(), ..Default::default() });
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(15));
    }
    emit(DevReloadEvent { stage: "no_answer".into(), id, files: names, detected_ms, sent_ms, ..Default::default() });
}

/// The watcher running now: its stop flag.
static WATCHER: std::sync::Mutex<Option<std::sync::Arc<std::sync::atomic::AtomicBool>>> = std::sync::Mutex::new(None);

fn write_watch_flag(user_zomboid_dir: &str, on: bool) {
    let lua = Path::new(user_zomboid_dir).join("Lua");
    if lua.is_dir() {
        let _ = crate::load_order::ini_parser::atomic_write(lua.join(WATCH_FILE), if on { "on=1\n" } else { "on=0\n" });
    }
}

/// Stops the watcher, and tells the bridge to stop looking at its queue.
pub fn stop_dev_watch(user_zomboid_dir: &str) {
    if let Ok(mut w) = WATCHER.lock() {
        if let Some(flag) = w.take() {
            flag.store(true, std::sync::atomic::Ordering::Relaxed);
        }
    }
    write_watch_flag(user_zomboid_dir, false);
}

/// Watches `root` until stopped (or another watch starts): every saved `.lua` under it is
/// reloaded in the game.
pub fn start_dev_watch<R: tauri::Runtime>(app: tauri::AppHandle<R>, root: PathBuf, user_zomboid_dir: String) -> Result<(), String> {
    if !root.is_dir() {
        return Err(format!("{} is not a folder.", root.display()));
    }
    stop_dev_watch(&user_zomboid_dir);
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    if let Ok(mut w) = WATCHER.lock() {
        *w = Some(stop.clone());
    }
    write_watch_flag(&user_zomboid_dir, true);
    std::thread::Builder::new()
        .name("dev-watch".into())
        .spawn(move || watch_loop(&app, &root, &user_zomboid_dir, &stop))
        .map_err(|e| e.to_string())?;
    Ok(())
}

/// Files changed since the last batch; `since` moves past them.
fn take_changes(root: &Path, since: &mut u64) -> Vec<ChangedFile> {
    let changed = changed_lua_files(root, *since);
    if let Some(latest) = changed.iter().map(|c| c.modified_ms).max() {
        *since = latest;
    }
    changed
}

#[cfg(windows)]
fn watch_loop<R: tauri::Runtime>(app: &tauri::AppHandle<R>, root: &Path, user_zomboid_dir: &str, stop: &std::sync::atomic::AtomicBool) {
    use std::os::windows::ffi::OsStrExt;
    use std::sync::atomic::Ordering;
    use windows_sys::Win32::Foundation::{INVALID_HANDLE_VALUE, WAIT_OBJECT_0};
    use windows_sys::Win32::Storage::FileSystem::{FindCloseChangeNotification, FindFirstChangeNotificationW, FindNextChangeNotification, FILE_NOTIFY_CHANGE_FILE_NAME, FILE_NOTIFY_CHANGE_LAST_WRITE};
    use windows_sys::Win32::System::Threading::WaitForSingleObject;

    let wide: Vec<u16> = root.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
    let handle = unsafe { FindFirstChangeNotificationW(wide.as_ptr(), 1, FILE_NOTIFY_CHANGE_LAST_WRITE | FILE_NOTIFY_CHANGE_FILE_NAME) };
    if handle == INVALID_HANDLE_VALUE {
        crate::diagnostics::error(format!("[Developer] cannot watch {}; looking every half second instead", root.display()));
        return poll_loop(app, root, user_zomboid_dir, stop);
    }
    let mut since = now_ms();
    while !stop.load(Ordering::Relaxed) {
        // Woken the moment something changes; every 200 ms otherwise, to see the stop flag.
        if unsafe { WaitForSingleObject(handle, 200) } != WAIT_OBJECT_0 {
            continue;
        }
        let detected = now_ms();
        unsafe { FindNextChangeNotification(handle) };
        // An editor saves in a few writes: wait until the folder is quiet for 100 ms.
        while unsafe { WaitForSingleObject(handle, 100) } == WAIT_OBJECT_0 {
            unsafe { FindNextChangeNotification(handle) };
        }
        let changed = take_changes(root, &mut since);
        if !changed.is_empty() && !stop.load(Ordering::Relaxed) {
            send_reload(app, user_zomboid_dir, with_dependents(root, changed), detected);
        }
    }
    unsafe { FindCloseChangeNotification(handle) };
}

#[cfg(not(windows))]
fn watch_loop<R: tauri::Runtime>(app: &tauri::AppHandle<R>, root: &Path, user_zomboid_dir: &str, stop: &std::sync::atomic::AtomicBool) {
    poll_loop(app, root, user_zomboid_dir, stop)
}

fn poll_loop<R: tauri::Runtime>(app: &tauri::AppHandle<R>, root: &Path, user_zomboid_dir: &str, stop: &std::sync::atomic::AtomicBool) {
    let mut since = now_ms();
    while !stop.load(std::sync::atomic::Ordering::Relaxed) {
        std::thread::sleep(std::time::Duration::from_millis(500));
        let changed = take_changes(root, &mut since);
        if !changed.is_empty() {
            send_reload(app, user_zomboid_dir, with_dependents(root, changed), now_ms());
        }
    }
}

#[cfg(test)]
mod reload_tests {
    use super::*;

    #[test]
    fn the_queue_names_the_request_and_each_file() {
        let text = reload_queue_text("reload_1", &["C:/Users/x/Zomboid/mods/My/42/media/lua/client/a.lua".into(), "D:/b\n.lua".into()]);
        assert_eq!(text, "id=reload_1\nfile=C:/Users/x/Zomboid/mods/My/42/media/lua/client/a.lua\nfile=D:/b.lua\n");
        let r: ReloadResult = serde_json::from_str(r#"{"id":"reload_1","applied_ms":1792590000123,"results":["ok a.lua","failed b.lua: nope"]}"#).unwrap();
        assert_eq!(r.results.len(), 2);
        assert_eq!(r.applied_ms, 1792590000123);
    }

    #[test]
    fn a_file_that_does_not_parse_is_not_sent() {
        let dir = std::env::temp_dir().join(format!("pzms-reload-check-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("media/lua/client")).unwrap();
        std::fs::write(dir.join("media/lua/client/good.lua"), "local x = 1\nreturn x\n").unwrap();
        std::fs::write(dir.join("media/lua/client/bad.lua"), "local x = = 1\n").unwrap();
        let (good, problems) = check_before_reload(&changed_lua_files(&dir, 0));
        assert_eq!(good.iter().map(|f| f.path.as_str()).collect::<Vec<_>>(), vec!["media/lua/client/good.lua"]);
        assert!(!problems.is_empty() && problems.iter().all(|p| p.path == "media/lua/client/bad.lua"), "{:?}", problems);
        assert_eq!(problems[0].line, Some(1));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_changed_module_brings_the_files_that_require_it_after_it() {
        assert_eq!(module_name("42/media/lua/client/PZTest/A.lua").as_deref(), Some("pztest/a"));
        assert_eq!(required_modules("local A = require \"PZTest/A\"
local B = require('PZTest/B.lua')
myrequire('no')
x.require('no')"), vec!["pztest/a", "pztest/b"]);
        let dir = std::env::temp_dir().join(format!("pzms-reload-deps-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let lua = dir.join("42/media/lua/client/PZTest");
        std::fs::create_dir_all(&lua).unwrap();
        std::fs::write(lua.join("A.lua"), "local M = {} return M").unwrap();
        std::fs::write(lua.join("B.lua"), "local A = require \"PZTest/A\"").unwrap();
        std::fs::write(lua.join("C.lua"), "local B = require(\"PZTest/B\")").unwrap();
        std::fs::write(lua.join("D.lua"), "print('alone')").unwrap();
        let a: Vec<ChangedFile> = changed_lua_files(&dir, 0).into_iter().filter(|f| f.path.ends_with("A.lua")).collect();
        let order: Vec<String> = with_dependents(&dir, a).into_iter().map(|f| f.path).collect();
        assert_eq!(order, vec!["42/media/lua/client/PZTest/A.lua", "42/media/lua/client/PZTest/B.lua", "42/media/lua/client/PZTest/C.lua"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn changed_files_are_those_touched_after_the_mark() {
        let dir = std::env::temp_dir().join(format!("pzms-reload-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("media/lua/client")).unwrap();
        std::fs::write(dir.join("media/lua/client/a.lua"), "x=1").unwrap();
        let all = changed_lua_files(&dir, 0);
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].path, "media/lua/client/a.lua");
        assert!(changed_lua_files(&dir, all[0].modified_ms + 1).is_empty());
        let mut since = 0;
        assert_eq!(take_changes(&dir, &mut since).len(), 1);
        assert!(take_changes(&dir, &mut since).is_empty(), "a batch is not sent twice");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

// ------------------------------------------------------------------------------------------
// A new mod, laid out the way Build 42 reads it.
//
// `Zomboid/mods/<folder>/42/mod.info` is what the installed game reads; `common/` holds what
// every build shares; a root `mod.info` is only there when Build 41 is wanted too. The poster
// is a plain PNG to replace. One Lua file prints the mod's name when the game loads it, so the
// Monitor shows it is alive on the first launch.
// ------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NewMod {
    /// The `id=` line: letters, digits, `_` and `-` only, since the game writes it into paths and the list.
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub author: String,
    #[serde(default)]
    pub description: String,
    /// "42" always; "41" too when the root mod.info is wanted.
    #[serde(default)]
    pub builds: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreatedMod {
    pub folder: String,
    pub files: Vec<String>,
}

/// A field that goes on one line of mod.info or a Lua comment: a line break would start another key.
fn one_line(s: &str) -> String {
    s.trim().chars().map(|c| if c.is_control() { ' ' } else { c }).collect()
}

/// Text that sits inside a double-quoted Lua string.
fn lua_quoted_body(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// The mod.info lines the game reads, in the order modders write them.
pub fn mod_info_text(m: &NewMod, build: &str) -> String {
    let mut lines = vec![format!("name={}", one_line(&m.name)), format!("id={}", m.id.trim())];
    for d in m.description.lines().map(str::trim).filter(|l| !l.is_empty()) {
        lines.push(format!("description={}", d));
    }
    lines.push("modversion=0.1".to_string());
    if !m.author.trim().is_empty() {
        lines.push(format!("authors={}", one_line(&m.author)));
    }
    lines.push("poster=poster.png".to_string());
    if build == "42" {
        lines.push("versionMin=42.0.0".to_string());
    }
    lines.join("\r\n") + "\r\n"
}

/// A plain dark PNG, `w` by `h`, for the poster until the modder has one.
fn plain_png(path: &Path, w: u32, h: u32) -> Result<(), String> {
    let file = std::fs::File::create(path).map_err(|e| e.to_string())?;
    let mut encoder = png::Encoder::new(std::io::BufWriter::new(file), w, h);
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(|e| e.to_string())?;
    let mut data = Vec::with_capacity((w * h * 3) as usize);
    for y in 0..h {
        for x in 0..w {
            // A frame of green around slate, so it reads as "a poster goes here" in the list.
            let edge = x < 8 || y < 8 || x + 8 >= w || y + 8 >= h;
            data.extend_from_slice(if edge { &[16, 185, 129] } else { &[15, 23, 42] });
        }
    }
    writer.write_image_data(&data).map_err(|e| e.to_string())
}

/// Makes the mod under `Zomboid/mods`, refusing an id that is not a plain name or a folder that exists.
pub fn create_mod(user_zomboid_dir: &str, m: &NewMod) -> Result<CreatedMod, String> {
    let id = m.id.trim();
    if id.is_empty() || !id.chars().all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-') {
        return Err("The id is the folder and the list entry: letters, digits, _ and - only.".to_string());
    }
    if m.name.trim().is_empty() {
        return Err("The mod needs a name.".to_string());
    }
    let mods_dir = crate::load_order::mod_info::get_existing_user_zomboid_dirs(user_zomboid_dir)
        .into_iter()
        .next()
        .map(|d| d.join("mods"))
        .ok_or_else(|| format!("Zomboid folder not found: {}", user_zomboid_dir))?;
    let root = mods_dir.join(id);
    if root.exists() {
        return Err(format!("There is already a folder {} in Zomboid/mods.", id));
    }
    let want_41 = m.builds.iter().any(|b| b == "41");
    let mut files = Vec::new();
    let mut put = |rel: &str, text: &str| -> Result<(), String> {
        let path = root.join(rel);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(&path, text).map_err(|e| format!("{}: {}", path.display(), e))?;
        files.push(rel.replace('\\', "/"));
        Ok(())
    };
    put("42/mod.info", &mod_info_text(m, "42"))?;
    let hello = format!(
        "-- {name}: loaded on every client (and in single player). Shared code goes in\r\n\
         -- common/media/lua/shared, server-side code in 42/media/lua/server.\r\n\
         print(\"[{id}] {quoted} loaded\")\r\n",
        name = one_line(&m.name),
        quoted = lua_quoted_body(&one_line(&m.name)),
        id = id
    );
    put(&format!("42/media/lua/client/{}_Client.lua", id), &hello)?;
    put("42/media/scripts/README.txt", "Item, recipe and vehicle scripts go here, as <name>.txt files in `module Base { ... }` blocks.\r\n")?;
    put("common/media/lua/shared/README.txt", "What every build shares: Lua loaded before client and server code, translations under translate/.\r\n")?;
    if want_41 {
        put("mod.info", &mod_info_text(m, "41"))?;
        put(&format!("media/lua/client/{}_Client.lua", id), &hello)?;
    }
    for dir in ["42/media/lua/shared", "42/media/lua/server", "common/media/lua/client", "common/media/lua/server"] {
        std::fs::create_dir_all(root.join(dir)).map_err(|e| e.to_string())?;
    }
    for rel in ["42/poster.png", "poster.png"] {
        plain_png(&root.join(rel), 256, 256)?;
        files.push(rel.to_string());
    }
    Ok(CreatedMod { folder: root.to_string_lossy().to_string(), files })
}

#[cfg(test)]
mod template_tests {
    use super::*;

    #[test]
    fn a_new_mod_has_the_build_42_layout_and_a_root_mod_info_only_with_41() {
        let dir = std::env::temp_dir().join(format!("pzms-template-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("mods")).unwrap();
        let z = dir.to_string_lossy().to_string();
        let m = NewMod { id: "MyMod".into(), name: "My Mod".into(), author: "Me".into(), description: "Line one\n\nLine two".into(), builds: vec!["42".into()] };
        let made = create_mod(&z, &m).unwrap();
        let root = dir.join("mods/MyMod");
        let info = std::fs::read_to_string(root.join("42/mod.info")).unwrap();
        assert!(info.starts_with("name=My Mod\r\nid=MyMod\r\ndescription=Line one\r\ndescription=Line two\r\n"), "{}", info);
        assert!(info.contains("authors=Me\r\n") && info.contains("versionMin=42.0.0\r\n"));
        assert!(!root.join("mod.info").exists(), "no root mod.info for a 42-only mod");
        assert!(root.join("42/media/lua/client/MyMod_Client.lua").is_file());
        assert!(root.join("common/media/lua/shared").is_dir());
        assert!(root.join("42/poster.png").metadata().unwrap().len() > 100);
        assert!(made.files.contains(&"42/mod.info".to_string()));
        assert!(looks_like_a_mod(&root));
        assert!(create_mod(&z, &m).is_err(), "the same folder twice is refused");
        assert!(create_mod(&z, &NewMod { id: "bad id".into(), ..m.clone() }).is_err());
        let both = NewMod { id: "Both".into(), builds: vec!["41".into(), "42".into()], ..m.clone() };
        create_mod(&z, &both).unwrap();
        let root41 = std::fs::read_to_string(dir.join("mods/Both/mod.info")).unwrap();
        assert!(!root41.contains("versionMin"), "the Build 41 mod.info carries no versionMin");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

// ------------------------------------------------------------------------------------------
// TileZed and WorldEd, The Indie Stone's own map editors. They come as a zip the modder puts
// anywhere, so the Studio looks in the usual places and in a folder the player names, and only
// ever starts those two programs.
// ------------------------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MapTools {
    pub tilezed: Option<String>,
    pub worlded: Option<String>,
    /// Where the Studio looked, for the screen to say when nothing was found.
    pub searched: Vec<String>,
}

const MAP_TOOL_EXES: [&str; 2] = ["TileZed.exe", "WorldEd.exe"];

/// The folders worth a look: the ones named (TileZed's, WorldEd's), then the usual unzip spots.
fn map_tool_dirs(configured: &[String]) -> Vec<PathBuf> {
    let mut dirs: Vec<PathBuf> = Vec::new();
    for c in configured.iter().map(|c| c.trim()).filter(|c| !c.is_empty()) {
        dirs.push(PathBuf::from(c));
    }
    let homes: Vec<PathBuf> = [std::env::var_os("USERPROFILE"), std::env::var_os("HOME")].into_iter().flatten().map(PathBuf::from).collect();
    for home in &homes {
        for sub in ["TileZed", "Tools/TileZed", "Documents/TileZed", "Desktop/TileZed", "Downloads/TileZed"] {
            dirs.push(home.join(sub));
        }
    }
    for root in ["C:\\TileZed", "D:\\TileZed", "C:\\Tools\\TileZed", "C:\\Program Files\\TileZed", "C:\\Program Files (x86)\\TileZed"] {
        dirs.push(PathBuf::from(root));
    }
    dirs
}

/// Looks for the two programs: in each folder and one level down (the zip unpacks into a
/// folder of its own more often than not).
pub fn find_map_tools(configured: &[String]) -> MapTools {
    let mut tools = MapTools::default();
    for dir in map_tool_dirs(configured) {
        tools.searched.push(dir.to_string_lossy().to_string());
        let mut candidates = vec![dir.clone()];
        if let Ok(entries) = std::fs::read_dir(&dir) {
            candidates.extend(entries.flatten().map(|e| e.path()).filter(|p| p.is_dir()));
        }
        for c in candidates {
            if tools.tilezed.is_none() && c.join(MAP_TOOL_EXES[0]).is_file() {
                tools.tilezed = Some(c.join(MAP_TOOL_EXES[0]).to_string_lossy().to_string());
            }
            if tools.worlded.is_none() && c.join(MAP_TOOL_EXES[1]).is_file() {
                tools.worlded = Some(c.join(MAP_TOOL_EXES[1]).to_string_lossy().to_string());
            }
        }
        if tools.tilezed.is_some() && tools.worlded.is_some() {
            break;
        }
    }
    tools
}

/// Starts TileZed or WorldEd — nothing else — from its own folder, so it finds its settings.
pub fn launch_map_tool(exe: &str) -> Result<(), String> {
    let path = Path::new(exe);
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    if !MAP_TOOL_EXES.iter().any(|e| e.eq_ignore_ascii_case(name)) {
        return Err("Only TileZed.exe and WorldEd.exe are started from here.".to_string());
    }
    if !path.is_file() {
        return Err(format!("{} is not there.", exe));
    }
    let mut cmd = std::process::Command::new(path);
    if let Some(dir) = path.parent() {
        cmd.current_dir(dir);
    }
    cmd.spawn().map(|_| ()).map_err(|e| format!("{} would not start: {}", name, e))
}

#[cfg(test)]
mod map_tool_tests {
    use super::*;

    #[test]
    fn the_named_folder_is_looked_in_first_and_one_level_down_and_only_the_two_exes_start() {
        let dir = std::env::temp_dir().join(format!("pzms-maptools-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("TileZed-2024")).unwrap();
        std::fs::write(dir.join("TileZed-2024/TileZed.exe"), b"").unwrap();
        let found = find_map_tools(&[dir.to_string_lossy().to_string()]);
        assert!(found.tilezed.as_deref().map(|p| p.ends_with("TileZed.exe")).unwrap_or(false), "{:?}", found);
        assert!(found.worlded.is_none());
        assert_eq!(found.searched[0], dir.to_string_lossy());
        assert!(launch_map_tool(&dir.join("TileZed-2024/notepad.exe").to_string_lossy()).is_err());
        assert!(launch_map_tool(&dir.join("nowhere/WorldEd.exe").to_string_lossy()).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
