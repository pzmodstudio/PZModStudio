//! Game functions that two or more active mods redefine, and whether they chain or one cuts the
//! other off.
//!
//! A mod changes a game function (`ISVehicleMenu.showRadialMenu`) either cooperatively — it keeps
//! the original (`local old = ISVehicleMenu.showRadialMenu`) and calls it from its own — or by
//! replacing it outright. Cooperative ones chain: each wraps the one before. A replacing one
//! drops whatever was there: every change made before it, by any mod, never runs, and the game
//! says nothing. Which one is "before" is the order the game runs the files in: the shared, then
//! client, then server phase; within a phase a mod file at a game file's own path runs in the
//! game's turn, then each mod's files in load order; and a definition made inside a function (an
//! event handler, an install() called at OnGameStart) runs after every file has loaded.
//!
//! Read lexically (comments and strings blanked, blocks counted), not with a full parser: fast
//! enough for 15,000 files, and kept per mod in memory until Steam updates it. Measured on a
//! 166-mod list: 58 functions shared, all chaining; the false positives a naive reading gives
//! (a local table named like a game one, `x = x or Original`, a wrap installed at OnGameStart)
//! are each covered by a test below.

use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

/// One redefinition of a game function in one mod file.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Redefinition {
    pub mod_id: String,
    pub mod_name: String,
    /// The file, from media/lua (`client/ISUI/ISFoo.lua`).
    pub file: String,
    pub line: u32,
    /// Keeps and calls the original (chains), or replaces it.
    pub keeps_original: bool,
    /// Made inside a function: runs after every file has loaded (from an event, an install()).
    pub deferred: bool,
    /// The file is at a game file's own path: it runs in the game's turn, before the mods'.
    pub replaces_game_file: bool,
    /// The event whose handler makes it (`OnGameStart`), when it is made inside one given to
    /// `Events.X.Add` directly. None with `deferred`: some function of the mod, run later.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub event: Option<String>,
    /// Not a definition of this function but `Table = {}`: the whole table made again, every
    /// function defined on it before gone.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub reset: bool,
    /// Its file may stop before it (`if getActivatedMods():contains("X") then return end` at file
    /// level above it): whether it runs depends on that test.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub conditional: bool,
}

/// What the game itself said about the function, read by the Studio Link in the running game:
/// the file and line of the function that is there once everything has loaded.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Checked {
    /// `menu` (after the main menu came up) or `game` (a few seconds into a loaded save).
    pub when: String,
    /// The mod whose file it is; None: the game's own file, or a file no active mod has.
    pub mod_id: Option<String>,
    pub file: String,
    pub line: u32,
    /// The mod the Studio expected last is the one the game has.
    pub agrees: bool,
    /// Unix seconds.
    pub at: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SharedFunction {
    /// `ISVehicleMenu.showRadialMenu` (a method `A:b` is written `A.b`).
    pub function: String,
    /// The mod whose table it is, for a function of a mod (not the game's) that other mods redefine.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub owner: Option<String>,
    /// In the order the game runs them.
    pub redefinitions: Vec<Redefinition>,
    /// The mod whose replacement is the last: everything before it is dropped. None: they chain.
    pub winner: Option<String>,
    /// The mods whose change never runs.
    pub cut: Vec<String>,
    /// Moving the winner before these mods would make them wrap it instead (all of them keep the
    /// original, load at file time, in the same phase): the fix is the order alone.
    pub fixed_by_moving_before: Option<String>,
    /// What the running game had, when the Studio Link has looked since this list was last read.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub checked: Option<Checked>,
}

// ------------------------------------------------------------------- reading one file

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Ident(String),
    Punct(char),
    /// `==`, `~=`, `<=`, `>=`, `..`: never an assignment, never a path.
    Op,
}

fn tokens(text: &str) -> Vec<(Tok, u32)> {
    let chars: Vec<char> = crate::api_lint::blank_comments_and_strings(text).chars().collect();
    let mut out = Vec::new();
    let mut line = 1u32;
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c == '\n' {
            line += 1;
            i += 1;
        } else if c.is_whitespace() {
            i += 1;
        } else if c.is_ascii_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            out.push((Tok::Ident(chars[start..i].iter().collect()), line));
        } else if c.is_ascii_digit() {
            while i < chars.len() && (chars[i].is_ascii_alphanumeric() || chars[i] == '.' || chars[i] == '_') {
                i += 1;
            }
            out.push((Tok::Op, line));
        } else {
            let next = chars.get(i + 1).copied().unwrap_or(' ');
            if matches!((c, next), ('=', '=') | ('~', '=') | ('<', '=') | ('>', '=') | ('.', '.')) {
                out.push((Tok::Op, line));
                i += 2;
            } else {
                out.push((Tok::Punct(c), line));
                i += 1;
            }
        }
    }
    out
}

/// What one file defines and reads: dotted names it assigns a function to (name, line, inside a
/// function or not), the dotted names it reads as values (keeping an original), the global names
/// it assigns at file level (a game file's tables), and the plain names it declares local.
/// Each definition is (name, line, inside a function, the event whose handler makes it).
#[derive(Debug, Default, Clone, PartialEq)]
pub struct FileScan {
    pub defs: Vec<(String, u32, bool, Option<String>)>,
    pub reads: BTreeSet<String>,
    pub globals: BTreeSet<String>,
    pub locals: BTreeSet<String>,
    /// Global tables made new at file level (`ISFoo = {}`, not `ISFoo = ISFoo or {}`), with the line.
    pub resets: Vec<(String, u32)>,
    /// The plain names this file makes functions: `local function f`, `function f(`,
    /// `[local] f = function`.
    pub fn_names: BTreeSet<String>,
    /// `A.b = f` where `f` is not a function of this file: a definition only when `f` is a
    /// function of the mod elsewhere (`button.title = label` is a field, not a function).
    pub aliases: Vec<((String, u32, bool, Option<String>), String)>,
    /// The first `return` at file level inside a block (`if X then return end`): the file may stop
    /// there, and what it defines below runs only when it does not.
    pub early_return: Option<u32>,
}

/// `Events.OnGameStart.Add(` just before token `i`: the event's name.
fn event_before(toks: &[(Tok, u32)], i: usize) -> Option<String> {
    if i < 6 {
        return None;
    }
    let t = |k: usize| &toks[i - k].0;
    match (t(6), t(5), t(4), t(3), t(2), t(1)) {
        (Tok::Ident(events), Tok::Punct('.'), Tok::Ident(name), Tok::Punct('.'), Tok::Ident(add), Tok::Punct('(')) if events == "Events" && add == "Add" => Some(name.clone()),
        _ => None,
    }
}

const KEYWORDS: [&str; 21] = [
    "and", "break", "do", "else", "elseif", "end", "false", "for", "function", "goto", "if", "in", "local", "nil", "not", "or", "repeat", "return", "then", "true", "until",
];

fn is_kw(s: &str) -> bool {
    KEYWORDS.contains(&s) || s == "while"
}

/// The dotted path starting at `i` (`A`, `.b`, `.c`, `:d`): its parts, whether a `:` closed it,
/// and the index after it.
fn path_at(toks: &[(Tok, u32)], i: usize) -> (Vec<String>, usize) {
    let mut parts = Vec::new();
    let Some((Tok::Ident(first), _)) = toks.get(i) else { return (parts, i) };
    if is_kw(first) {
        return (parts, i);
    }
    parts.push(first.clone());
    let mut j = i + 1;
    while let (Some((Tok::Punct(p), _)), Some((Tok::Ident(n), _))) = (toks.get(j), toks.get(j + 1)) {
        if (*p != '.' && *p != ':') || is_kw(n) {
            break;
        }
        parts.push(n.clone());
        j += 2;
        if *p == ':' {
            break;
        }
    }
    (parts, j)
}

pub fn scan(text: &str) -> FileScan {
    let toks = tokens(text);
    let mut scan = FileScan::default();
    // Open blocks: whether each is a function body, and the name an `if not X then` guards (a
    // fallback defined only when the game's is missing: never a replacement of the game's).
    // Each block: (a function body, the name an `if not X` guards, the event a function was given
    // to inline, the function's own name: `local function install()`, `install = function`).
    let mut stack: Vec<(bool, Option<String>, Option<String>, Option<String>)> = Vec::new();
    // The name the next `function` gets (`local install = function`).
    let mut next_fn_name: Option<String> = None;
    // Definitions made inside a named function (index in defs, the function's name), and how that
    // function is run: called at file level (`install()`) or given to an event by name
    // (`Events.OnGameBoot.Add(install)`).
    let mut owned: Vec<(usize, String)> = Vec::new();
    let mut owned_aliases: Vec<(usize, String)> = Vec::new();
    let mut called_at_file_level: BTreeSet<String> = BTreeSet::new();
    let mut handler_of: HashMap<String, String> = HashMap::new();
    // A named function's parameters, the names read inside one (owner), and its calls at file
    // level with the tables they pass (`wrapAction(ISReloadWeaponAction)`): what it does to a
    // field of a parameter it does to the table passed there.
    let mut params_of: HashMap<String, Vec<String>> = HashMap::new();
    let mut owned_reads: Vec<(String, String)> = Vec::new();
    let mut file_calls: Vec<(String, Vec<Option<String>>)> = Vec::new();
    // `while`/`for` open their block themselves: their `do` opens nothing more.
    let mut loop_do_pending = 0usize;
    // Inside a table constructor `{ ... }`: `name = value` there is a field, not a global.
    let mut braces = 0usize;
    let mut i = 0;
    while i < toks.len() {
        let in_function = stack.iter().any(|f| f.0);
        // The event the outermost function block was given to, if any: when what it defines runs.
        let event = stack.iter().find(|f| f.0).and_then(|f| f.2.clone());
        let owner = stack.iter().find(|f| f.0).and_then(|f| f.3.clone());
        let guarded = |name: &str| stack.iter().filter_map(|b| b.1.as_deref()).any(|g| name == g || name.starts_with(&format!("{}.", g)));
        let prev_is_path_link = i > 0 && matches!(toks[i - 1].0, Tok::Punct('.') | Tok::Punct(':'));
        match &toks[i].0 {
            Tok::Ident(w) if w == "function" => {
                let (parts, after) = path_at(&toks, i + 1);
                if parts.len() >= 2 && matches!(toks.get(after), Some((Tok::Punct('('), _))) {
                    let name = parts.join(".");
                    if !guarded(&name) {
                        if let (true, None, Some(o)) = (in_function, &event, &owner) {
                            owned.push((scan.defs.len(), o.clone()));
                        }
                        scan.defs.push((name, toks[i].1, in_function, event.clone()));
                    }
                } else if parts.len() == 1 && !in_function && !(i > 0 && toks[i - 1].0 == Tok::Ident("local".into())) {
                    // `function Foo()` at file level: a global function (a game file's too).
                    scan.globals.insert(parts[0].clone());
                }
                if parts.len() == 1 {
                    scan.fn_names.insert(parts[0].clone());
                }
                // Its parameters are locals: `local function base(def) def.x = ... end` sets a
                // field of what it was given, not of a global table `def`.
                let mut params: Vec<String> = Vec::new();
                if matches!(toks.get(after), Some((Tok::Punct('('), _))) {
                    let mut j = after + 1;
                    while let Some((tok, _)) = toks.get(j) {
                        match tok {
                            Tok::Ident(n) if !is_kw(n) => {
                                scan.locals.insert(n.clone());
                                params.push(n.clone());
                            }
                            Tok::Punct(')') => break,
                            _ => {}
                        }
                        j += 1;
                    }
                }
                let fn_name = if parts.len() == 1 { Some(parts[0].clone()) } else { next_fn_name.take() };
                next_fn_name = None;
                if let (Some(n), false) = (&fn_name, in_function || params.is_empty()) {
                    params_of.insert(n.clone(), params);
                }
                stack.push((true, None, event_before(&toks, i), fn_name));
                i += 1;
                continue;
            }
            Tok::Ident(w) if w == "local" => {
                // `local a, b = ...` / `local function a`: names that hide a global of that name here.
                let mut j = i + 1;
                if matches!(toks.get(j), Some((Tok::Ident(f), _)) if f == "function") {
                    j += 1;
                    if let Some((Tok::Ident(n), _)) = toks.get(j) {
                        scan.fn_names.insert(n.clone());
                    }
                }
                while let Some((Tok::Ident(n), _)) = toks.get(j) {
                    if is_kw(n) {
                        break;
                    }
                    scan.locals.insert(n.clone());
                    if matches!(toks.get(j + 1), Some((Tok::Punct(','), _))) {
                        j += 2;
                    } else {
                        break;
                    }
                }
                i += 1;
                continue;
            }
            Tok::Ident(w) if w == "if" => {
                // `if not A.b then` / `if A.b == nil then`: what follows defines A.b only when missing.
                let guard = match toks.get(i + 1) {
                    Some((Tok::Ident(n), _)) if n == "not" => {
                        let (p, end) = path_at(&toks, i + 2);
                        (!p.is_empty() && matches!(toks.get(end), Some((Tok::Ident(t), _)) if t == "then")).then(|| p.join("."))
                    }
                    _ => {
                        let (p, end) = path_at(&toks, i + 1);
                        (!p.is_empty() && matches!(toks.get(end), Some((Tok::Op, _))) && matches!(toks.get(end + 1), Some((Tok::Ident(n), _)) if n == "nil")).then(|| p.join("."))
                    }
                };
                stack.push((false, guard, None, None));
            }
            Tok::Ident(w) if w == "repeat" => stack.push((false, None, None, None)),
            Tok::Ident(w) if w == "while" || w == "for" => {
                stack.push((false, None, None, None));
                loop_do_pending += 1;
            }
            Tok::Ident(w) if w == "do" => {
                if loop_do_pending > 0 {
                    loop_do_pending -= 1;
                } else {
                    stack.push((false, None, None, None));
                }
            }
            Tok::Ident(w) if w == "end" || w == "until" => {
                stack.pop();
            }
            Tok::Ident(w) if w == "return" && !in_function && !stack.is_empty() && scan.early_return.is_none() => {
                scan.early_return = Some(toks[i].1);
            }
            Tok::Punct('{') => braces += 1,
            Tok::Punct('}') => braces = braces.saturating_sub(1),
            Tok::Ident(w) if !is_kw(w) && !prev_is_path_link => {
                let (parts, after) = path_at(&toks, i);
                let assigned = matches!(toks.get(after), Some((Tok::Punct('='), _)));
                if parts.len() >= 2 {
                    let name = parts.join(".");
                    if assigned {
                        // `A.b = function`, or `A.b = myNewFunction` (a local function given its place).
                        match toks.get(after + 1) {
                            Some((Tok::Ident(f), _)) if f == "function" => {
                                if !guarded(&name) {
                                    if let (true, None, Some(o)) = (in_function, &event, &owner) {
                                        owned.push((scan.defs.len(), o.clone()));
                                    }
                                    scan.defs.push((name, toks[i].1, in_function, event.clone()));
                                }
                            }
                            Some((Tok::Ident(v), _)) if !is_kw(v) => {
                                let (value, value_end) = path_at(&toks, after + 1);
                                let next = toks.get(value_end);
                                // `A.b = mine`, not `A.b = x or y`, `A.b = f(...)`, `A.b = t.c`.
                                let continues = matches!(next, Some((Tok::Ident(o), _)) if o == "or" || o == "and");
                                let used_further = matches!(next, Some((Tok::Punct('(' | '[' | '.' | ':'), _)));
                                if value.len() == 1 && !continues && !used_further && !guarded(&name) {
                                    if let (true, None, Some(o)) = (in_function, &event, &owner) {
                                        owned_aliases.push((scan.aliases.len(), o.clone()));
                                    }
                                    scan.aliases.push(((name, toks[i].1, in_function, event.clone()), value[0].clone()));
                                }
                            }
                            _ => {}
                        }
                    } else if !matches!(toks.get(after), Some((Tok::Punct('(' | '{'), _))) {
                        // Read as a value, not called: kept (`local old = A.b`, `x = x or A.b`).
                        if let (true, None, Some(o)) = (in_function, &event, &owner) {
                            owned_reads.push((name.clone(), o.clone()));
                        }
                        scan.reads.insert(name);
                    } else if parts.len() == 3 && parts[0] == "Events" && parts[2] == "Add" && !in_function {
                        // `Events.OnGameBoot.Add(install)`: install runs at that event.
                        if let (Some((Tok::Punct('('), _)), Some((Tok::Ident(h), _)), Some((Tok::Punct(')'), _))) = (toks.get(after), toks.get(after + 1), toks.get(after + 2)) {
                            handler_of.insert(h.clone(), parts[1].clone());
                        }
                    }
                } else if parts.len() == 1 && !assigned && !in_function && matches!(toks.get(after), Some((Tok::Punct('('), _))) {
                    // `install()` at file level: what that function defines is made now.
                    called_at_file_level.insert(parts[0].clone());
                    // Its arguments, each a table's name when it is only that (`wrap(ISFoo)`,
                    // `wrap(Mod.Action, f)`), None otherwise.
                    let mut args: Vec<Option<String>> = Vec::new();
                    let mut j = after + 1;
                    let mut depth = 0usize;
                    let mut start = j;
                    while let Some((tok, _)) = toks.get(j) {
                        let closes = depth == 0 && *tok == Tok::Punct(')');
                        if (depth == 0 && *tok == Tok::Punct(',')) || closes {
                            let (p, end) = path_at(&toks, start);
                            args.push((!p.is_empty() && end == j).then(|| p.join(".")));
                            if closes {
                                break;
                            }
                            start = j + 1;
                        } else if matches!(tok, Tok::Punct('(' | '{' | '[')) || matches!(tok, Tok::Ident(w) if ["function", "if", "do", "repeat"].contains(&w.as_str())) {
                            depth += 1;
                        } else if matches!(tok, Tok::Punct(')' | '}' | ']')) || matches!(tok, Tok::Ident(w) if w == "end" || w == "until") {
                            depth = depth.saturating_sub(1);
                        }
                        j += 1;
                    }
                    if args.iter().any(Option::is_some) {
                        file_calls.push((parts[0].clone(), args));
                    }
                } else if parts.len() == 1 && assigned && matches!(toks.get(after + 1), Some((Tok::Ident(f), _)) if f == "function") {
                    // `f = function` / `local f = function`: a function of this file.
                    scan.fn_names.insert(parts[0].clone());
                    next_fn_name = Some(parts[0].clone());
                    if !in_function && stack.is_empty() && braces == 0 {
                        scan.globals.insert(parts[0].clone());
                    }
                } else if parts.len() == 1 && assigned && !in_function && stack.is_empty() && braces == 0 {
                    scan.globals.insert(parts[0].clone());
                    let local = i > 0 && toks[i - 1].0 == Tok::Ident("local".into());
                    if !local && matches!(toks.get(after + 1), Some((Tok::Punct('{'), _))) {
                        scan.resets.push((parts[0].clone(), toks[i].1));
                    }
                }
                i = after.max(i + 1);
                continue;
            }
            _ => {}
        }
        i += 1;
    }
    // A definition inside a named function runs when that function does: at file level when the
    // file calls it there, at an event when it is given to one by name.
    // Called at file level first: it runs then, whatever event it is also given to afterwards.
    let when = |name: &str, def: &mut (String, u32, bool, Option<String>)| {
        if called_at_file_level.contains(name) {
            def.2 = false;
        } else if let Some(event) = handler_of.get(name) {
            def.3 = Some(event.clone());
        }
    };
    // `local function wrap(class) class.start = function ... end end` then `wrap(ISFoo)` at file
    // level: ISFoo.start is defined then, keeping the original when the function reads it.
    let mut passed: Vec<(String, u32, bool, Option<String>)> = Vec::new();
    for (function, args) in &file_calls {
        let Some(params) = params_of.get(function) else { continue };
        let through = |name: &str| -> Vec<String> {
            let (root, rest) = name.split_once('.').unwrap_or((name, ""));
            params
                .iter()
                .zip(args.iter())
                .filter(|(p, _)| p.as_str() == root && !rest.is_empty())
                .filter_map(|(_, a)| a.as_ref().map(|t| format!("{}.{}", t, rest)))
                .collect()
        };
        for (at, owner) in &owned {
            if owner == function {
                let (name, line, _, _) = &scan.defs[*at];
                for real in through(name) {
                    passed.push((real, *line, false, None));
                }
            }
        }
        for (name, owner) in &owned_reads {
            if owner == function {
                scan.reads.extend(through(name));
            }
        }
    }
    for (at, name) in owned {
        when(&name, &mut scan.defs[at]);
    }
    for def in passed {
        if !scan.defs.contains(&def) {
            scan.defs.push(def);
        }
    }
    for (at, name) in owned_aliases {
        when(&name, &mut scan.aliases[at].0);
    }
    // `A.b = f` with `f` a function of this file: a definition; the rest wait for the mod's.
    let aliases = std::mem::take(&mut scan.aliases);
    for (def, value) in aliases {
        if scan.fn_names.contains(&value) {
            scan.defs.push(def);
        } else if !scan.locals.contains(&value) {
            // A local of this file that is not a function (`local tip = {}`) is a value; a
            // name from elsewhere may be a function another file of the mod makes.
            scan.aliases.push((def, value));
        }
    }
    scan.defs.sort_by_key(|d| d.1);
    scan
}

// ------------------------------------------------------------------- the game and the mods

/// What the game's own Lua defines: its functions (dotted names) and its global tables, and the
/// paths of its files (a mod file at one of them replaces it).
#[derive(Debug, Default)]
pub struct GameLua {
    pub functions: HashSet<String>,
    pub globals: HashSet<String>,
    pub files: HashSet<String>,
}

fn lua_files_under(dir: &Path) -> Vec<PathBuf> {
    walkdir::WalkDir::new(dir)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_file() && e.path().extension().is_some_and(|x| x.eq_ignore_ascii_case("lua")))
        .map(|e| e.into_path())
        .collect()
}

/// `client/ISUI/ISFoo.lua` for a file somewhere under a `media/lua` folder.
fn rel_under_lua(path: &Path) -> Option<String> {
    let s = path.to_string_lossy().replace('\\', "/");
    let at = s.to_lowercase().rfind("/media/lua/")?;
    Some(s[at + "/media/lua/".len()..].to_string())
}

type GameKey = (PathBuf, u64);
static GAME: Mutex<Option<(GameKey, Arc<GameLua>)>> = Mutex::new(None);

/// The installed game's Lua, read once per build (the jar's size tells a new one).
pub fn game_lua(install_dir: &Path) -> Arc<GameLua> {
    let key = (install_dir.to_path_buf(), std::fs::metadata(install_dir.join("projectzomboid.jar")).map(|m| m.len()).unwrap_or(0));
    if let Some((k, g)) = GAME.lock().unwrap_or_else(|p| p.into_inner()).as_ref() {
        if *k == key {
            return g.clone();
        }
    }
    let mut game = GameLua::default();
    for f in lua_files_under(&install_dir.join("media").join("lua")) {
        let Some(rel) = rel_under_lua(&f) else { continue };
        game.files.insert(rel.to_lowercase());
        let Ok(text) = std::fs::read_to_string(&f) else { continue };
        let s = scan(&text);
        game.functions.extend(s.defs.into_iter().filter(|d| !d.2).map(|d| d.0));
        game.globals.extend(s.globals);
    }
    let game = Arc::new(game);
    *GAME.lock().unwrap_or_else(|p| p.into_inner()) = Some((key, game.clone()));
    game
}

/// One mod file's redefinition: (function, file, line, keeps, deferred, replaces game file, event).
type ModDef = (String, String, u32, bool, bool, bool, Option<String>);

/// What one mod does to functions, read once per mod version: its redefinitions of the game's
/// functions, of other names (functions of a mod's table), the tables it makes new at file level
/// (table, file, line, replaces game file) and the global tables it makes.
#[derive(Debug, Default, Clone, Serialize, serde::Deserialize)]
struct ModDefs {
    game: Vec<ModDef>,
    other: Vec<ModDef>,
    resets: Vec<(String, String, u32, bool)>,
    globals: Vec<String>,
    /// Its Lua files the build reads, lowercased from media/lua: where another mod later in the
    /// list has the same path, the game runs that one's copy and never this one.
    #[serde(default)]
    files: Vec<String>,
    /// Files that may stop early (path as in the redefinitions, line of the `return`).
    #[serde(default)]
    returns: Vec<(String, u32)>,
}

type ModKey = (String, u64, Option<u64>, Option<u32>);
static MODS: Mutex<Option<HashMap<ModKey, Arc<ModDefs>>>> = Mutex::new(None);

fn mod_defs(m: &crate::catalog::CatalogMod, game: &GameLua, version: Option<u32>) -> Arc<ModDefs> {
    let key = m.workshop_state.as_ref().and_then(|s| s.installed_time.map(|t| (m.root_dir.clone(), t, s.size, version)));
    if let Some(key) = &key {
        if let Some(found) = MODS.lock().unwrap_or_else(|p| p.into_inner()).as_ref().and_then(|c| c.get(key)) {
            return found.clone();
        }
    }
    // Kept on disk from an earlier start (mod_cache), until Steam updates the mod or the game changes.
    let disk_key = crate::mod_cache::workshop_key(m, &format!("overrides7 {:?} {}", version, game.functions.len()));
    if let Some(found) = disk_key.as_deref().and_then(|k| crate::mod_cache::load::<ModDefs>("overrides", k)) {
        let found = Arc::new(found);
        if let Some(key) = key {
            MODS.lock().unwrap_or_else(|p| p.into_inner()).get_or_insert_with(HashMap::new).insert(key, found.clone());
        }
        return found;
    }
    // The build's layers, the version folder's copy of a path replacing common/'s.
    // Paths are the same whatever their case (the game reads them so on Windows).
    let mut files: BTreeMap<String, (String, PathBuf)> = BTreeMap::new();
    for layer in crate::load_order::mod_info::media_layers_for_build(Path::new(&m.root_dir), version) {
        for f in lua_files_under(&layer.join("media").join("lua")) {
            if let Some(rel) = rel_under_lua(&f) {
                files.insert(rel.to_lowercase(), (rel, f));
            }
        }
    }
    let mut out = ModDefs::default();
    let mut globals = BTreeSet::new();
    let mut scans: Vec<(String, FileScan)> = Vec::new();
    for (_, (rel, path)) in files {
        out.files.push(rel.to_lowercase());
        let Ok(text) = std::fs::read_to_string(&path) else { continue };
        scans.push((rel, scan(&text)));
    }
    // `A.b = f` with `f` a function another file of the mod makes.
    let mod_fns: BTreeSet<String> = scans.iter().flat_map(|(_, s)| s.fn_names.iter().cloned()).collect();
    for (_, s) in scans.iter_mut() {
        let aliases = std::mem::take(&mut s.aliases);
        for (def, value) in aliases {
            if mod_fns.contains(&value) {
                s.defs.push(def);
            }
        }
    }
    for (rel, s) in scans {
        if let Some(line) = s.early_return {
            out.returns.push((rel.clone(), line));
        }
        let at_game_path = game.files.contains(&rel.to_lowercase());
        for (name, line, deferred, event) in &s.defs {
            let root = name.split('.').next().unwrap_or("");
            if s.locals.contains(root) {
                continue; // a local table of the same name, not the global one
            }
            let def = (name.clone(), rel.clone(), *line, s.reads.contains(name), *deferred, at_game_path, event.clone());
            if game.functions.contains(name) && game.globals.contains(root) {
                out.game.push(def);
            } else if !game.globals.contains(root) {
                out.other.push(def);
            }
        }
        for (table, line) in &s.resets {
            if !s.locals.contains(table) {
                out.resets.push((table.clone(), rel.clone(), *line, at_game_path));
            }
        }
        globals.extend(s.globals.iter().filter(|g| !s.locals.contains(*g)).cloned());
    }
    out.globals = globals.into_iter().collect();
    if let Some(k) = disk_key.as_deref() {
        crate::mod_cache::save("overrides", k, &out);
    }
    let out = Arc::new(out);
    if let Some(key) = key {
        MODS.lock().unwrap_or_else(|p| p.into_inner()).get_or_insert_with(HashMap::new).insert(key, out.clone());
    }
    out
}

fn phase(file: &str) -> u8 {
    match file.split('/').next().unwrap_or("").to_lowercase().as_str() {
        "shared" => 0,
        "client" => 1,
        "server" => 2,
        _ => 3,
    }
}

/// When a definition made in an event's handler runs: after every file has loaded, in the order
/// the game fires the events (boot, the menu, then loading a save). Any other function of the mod
/// that defines it (an install() called from somewhere) is said to run last.
fn when_rank(r: &Redefinition) -> u8 {
    if !r.deferred {
        return 0;
    }
    match r.event.as_deref() {
        Some("OnGameBoot") => 1,
        Some("OnMainMenuEnter") => 2,
        Some("OnInitWorld") => 3,
        Some("OnInitGlobalModData") => 4,
        Some("OnLoad") | Some("OnNewGame") => 5,
        Some("OnGameStart") => 6,
        Some("OnCreatePlayer") | Some("OnCreateLivingCharacter") => 7,
        _ => 9,
    }
}

/// Made in time for the main menu: file time, boot and the menu's own event (a save's come later).
pub fn runs_by_menu(r: &Redefinition) -> bool {
    // The server/ folder loads only when a save does (GameLoadingState): not by the menu.
    when_rank(r) <= 2 && phase(&r.file) != 2
}

/// The order the game runs a redefinition in: file time first, then handlers by event; then
/// phase; a file at a game path in the game's turn; then load order; then path and line.
fn run_order(r: &Redefinition, position: usize) -> (u8, u8, usize, String, u32) {
    (when_rank(r), phase(&r.file), if r.replaces_game_file { 0 } else { position + 1 }, r.file.to_lowercase(), r.line)
}

fn plain(id: &str) -> &str {
    id.trim().trim_start_matches('\\')
}

/// Every function two or more active mods redefine, in run order, with who cuts whom: the game's
/// own, and those of a mod's table that another mod redefines without either requiring the other
/// (a mod that requires the other builds on it on purpose).
pub fn shared_functions(install_dir: &Path, catalog: &crate::catalog::ModCatalog, active: &[String]) -> Vec<SharedFunction> {
    let game = game_lua(install_dir);
    if game.functions.is_empty() {
        return Vec::new();
    }
    let version = catalog.game_version_int();
    // The order the game runs the files in is its load order, not the list's: each mod after what
    // it requires (a requirement further down, or off, loads first), a mod it cannot load left out.
    let loaded = crate::catalog::game_load_order(catalog, active);
    let active = loaded.as_slice();
    let mut by_function: BTreeMap<String, Vec<(usize, Redefinition)>> = BTreeMap::new();
    let mut other: BTreeMap<String, Vec<(usize, Redefinition)>> = BTreeMap::new();
    let mut resets: Vec<(usize, String, Redefinition)> = Vec::new();
    // The mods that make each global table of their own at file level, in load order.
    let mut makers: HashMap<String, Vec<String>> = HashMap::new();
    let mut requires: HashMap<String, HashSet<String>> = HashMap::new();
    // Which mod's copy of each Lua path the game runs: the last in the list that has it (the
    // game logs `mod "B" overrides media/lua/...` and loads only B's file). The Studio's packages
    // count here too: a file they carry replaces the mod's.
    let mut runs_copy_of: HashMap<String, String> = HashMap::new();
    let mut listed = HashSet::new();
    for entry in active {
        let Some(m) = catalog.resolve(entry.trim()) else { continue };
        if m.loadability != crate::catalog::Loadability::Loadable || !listed.insert(m.id.clone()) {
            continue;
        }
        for f in &mod_defs(m, &game, version).files {
            runs_copy_of.insert(f.clone(), m.id.clone());
        }
    }
    let mut seen = HashSet::new();
    for (position, entry) in active.iter().enumerate() {
        if entry.starts_with("Z_PZModStudio_") {
            continue;
        }
        let Some(m) = catalog.resolve(entry.trim()) else { continue };
        if m.loadability != crate::catalog::Loadability::Loadable || !seen.insert(m.id.clone()) {
            continue;
        }
        requires.insert(m.id.clone(), m.info.require.iter().map(|r| plain(r).to_string()).collect());
        let defs = mod_defs(m, &game, version);
        // A file of this mod that a later mod's copy replaces never runs: nothing in it counts.
        let runs = |file: &str| runs_copy_of.get(&file.to_lowercase()).is_none_or(|owner| *owner == m.id);
        let stops_before = |file: &str, line: u32| defs.returns.iter().any(|(f, at)| f == file && *at < line);
        let redef = |d: &ModDef| Redefinition {
            mod_id: m.id.clone(),
            mod_name: m.name.clone(),
            file: d.1.clone(),
            line: d.2,
            keeps_original: d.3,
            deferred: d.4,
            replaces_game_file: d.5,
            event: d.6.clone(),
            reset: false,
            conditional: stops_before(&d.1, d.2),
        };
        for d in defs.game.iter().filter(|d| runs(&d.1)) {
            by_function.entry(d.0.clone()).or_default().push((position, redef(d)));
        }
        for d in defs.other.iter().filter(|d| runs(&d.1)) {
            other.entry(d.0.clone()).or_default().push((position, redef(d)));
        }
        for (table, file, line, at_game_path) in defs.resets.iter().filter(|r| runs(&r.1)) {
            let r = Redefinition {
                mod_id: m.id.clone(),
                mod_name: m.name.clone(),
                file: file.clone(),
                line: *line,
                keeps_original: false,
                deferred: false,
                replaces_game_file: *at_game_path,
                event: None,
                reset: true,
                conditional: stops_before(file, *line),
            };
            resets.push((position, table.clone(), r));
        }
        for g in &defs.globals {
            makers.entry(g.clone()).or_default().push(m.id.clone());
        }
    }
    // A function of a mod's table that another mod redefines: kept when some active mod makes the
    // table and two of the mods redefining it have no requirement between them.
    // Through a library in between too: an addon of a framework it reaches by another library.
    let reaches = |from: &str, to: &str| -> bool {
        let mut seen: HashSet<&str> = HashSet::new();
        let mut stack = vec![from];
        while let Some(m) = stack.pop() {
            if !seen.insert(m) {
                continue;
            }
            for r in requires.get(m).into_iter().flatten() {
                if r == to {
                    return true;
                }
                stack.push(r.as_str());
            }
        }
        false
    };
    let related = |a: &str, b: &str| reaches(a, b) || reaches(b, a);
    let mut owner_of: HashMap<String, String> = HashMap::new();
    for (name, defs) in other {
        let root = name.split('.').next().unwrap_or("");
        let Some(made_by) = makers.get(root) else { continue };
        let mods: Vec<&str> = defs.iter().map(|(_, r)| r.mod_id.as_str()).collect::<BTreeSet<_>>().into_iter().collect();
        let unrelated = mods.iter().enumerate().any(|(i, a)| mods[i + 1..].iter().any(|b| !related(a, b)));
        if mods.len() >= 2 && unrelated {
            if let Some(o) = made_by.first() {
                owner_of.insert(name.clone(), o.clone());
            }
            by_function.insert(name, defs);
        }
    }
    // A table made new wipes what was defined on it before: a replacement of each of its
    // functions that another mod redefines, in its file's turn.
    let mut under: HashMap<String, Vec<String>> = HashMap::new();
    for name in by_function.keys() {
        if let Some((root, _)) = name.split_once('.') {
            under.entry(root.to_string()).or_default().push(name.clone());
        }
    }
    for (position, table, r) in resets {
        for name in under.get(&table).into_iter().flatten() {
            if let Some(defs) = by_function.get_mut(name) {
                if defs.iter().any(|(_, d)| d.mod_id != r.mod_id) {
                    defs.push((position, r.clone()));
                }
            }
        }
    }
    by_function
        .into_iter()
        .filter(|(_, defs)| defs.iter().map(|(_, r)| &r.mod_id).collect::<HashSet<_>>().len() >= 2)
        .map(|(function, mut defs)| {
            defs.sort_by(|a, b| run_order(&a.1, a.0).cmp(&run_order(&b.1, b.0)));
            let owner = owner_of.remove(&function);
            let mut v = verdict(function, defs);
            v.owner = owner;
            v
        })
        .collect()
}

/// Whether the cut follows from the load order alone: the winner's replacement and every change
/// it drops are made while the files load (not in a function run later, an event's handler or an
/// install(), which may run under conditions), in files that do not stop early. Measured on three
/// collections (2026-11-18) these were each confirmed by reading the code; the others depend on
/// what runs in the game.
pub fn cut_is_certain(f: &SharedFunction) -> bool {
    let Some(winner) = f.winner.as_deref() else { return false };
    let Some(at) = f.redefinitions.iter().rposition(|r| r.mod_id == winner && !r.keeps_original) else { return false };
    let sure = |r: &Redefinition| !r.deferred && !r.conditional;
    sure(&f.redefinitions[at]) && f.redefinitions[..at].iter().filter(|r| f.cut.contains(&r.mod_id)).all(sure)
}

// Who cuts whom, from redefinitions already in run order (with their load positions).
pub fn verdict(function: String, defs: Vec<(usize, Redefinition)>) -> SharedFunction {
    let last_replacing = defs.iter().rposition(|(_, r)| !r.keeps_original);
    let (winner, cut, fix) = match last_replacing {
        Some(at) if at > 0 => {
            let w = &defs[at].1;
            // A mod that also redefines it after the winner (in an event, later) wraps the winner: not cut.
            let after: HashSet<&str> = defs[at + 1..].iter().map(|(_, r)| r.mod_id.as_str()).collect();
            let before: Vec<&(usize, Redefinition)> = defs[..at].iter().filter(|(_, r)| r.mod_id != w.mod_id && !after.contains(r.mod_id.as_str())).collect();
            let mut cut: Vec<String> = Vec::new();
            for (_, r) in &before {
                if !cut.contains(&r.mod_id) {
                    cut.push(r.mod_id.clone());
                }
            }
            // The order alone fixes it when the winner runs at file time, not at a game path, and
            // every mod it cuts keeps the original, at file time, in the winner's phase: moved
            // before them, it runs first and they wrap it.
            let movable = !w.deferred
                && !w.replaces_game_file
                && before.iter().all(|(_, r)| r.keeps_original && !r.deferred && !r.replaces_game_file && phase(&r.file) == phase(&w.file));
            let first = before.iter().min_by_key(|(p, _)| *p).map(|(_, r)| r.mod_id.clone());
            if cut.is_empty() {
                (None, cut, None)
            } else {
                (Some(w.mod_id.clone()), cut, if movable { first } else { None })
            }
        }
        _ => (None, Vec::new(), None),
    };
    SharedFunction { function, owner: None, redefinitions: defs.into_iter().map(|(_, r)| r).collect(), winner, cut, fixed_by_moving_before: fix, checked: None }
}

// ------------------------------------------------------------------- checked in the running game

/// app -> game: the functions to look at (`list=<key>`, `fn=<name>` lines). The Studio Link reads
/// it a few seconds after the main menu and after a save loads, and writes what it found.
pub const PROBE_FILE: &str = "PZModStudio_LinkProbe.txt";
/// game -> app: `list=`, `at=`, then `fn=<name>\t<file>\t<line>` (`-` and 0: not a Lua function).
pub const PROBE_MENU_FILE: &str = "PZModStudio_LinkProbeMenu.txt";
pub const PROBE_GAME_FILE: &str = "PZModStudio_LinkProbeGame.txt";
const PROBE_MAX: usize = 400;

/// A short key of the functions asked about: a reply for another list is not this list's.
/// The functions asked about, at most PROBE_MAX: those with a mod cut off first, then the game's.
fn probe_names(shared: &[SharedFunction]) -> Vec<&str> {
    let mut list: Vec<&SharedFunction> = shared.iter().collect();
    list.sort_by_key(|f| (f.winner.is_none(), f.owner.is_some()));
    list.into_iter().take(PROBE_MAX).map(|f| f.function.as_str()).collect()
}

fn list_key(shared: &[SharedFunction]) -> String {
    let mut h: u64 = 0xcbf29ce484222325;
    for name in probe_names(shared) {
        // With the mod expected last: after a reorder a reply about the old order is not this one's.
        let last = shared.iter().find(|f| f.function == name).and_then(|f| f.redefinitions.last()).map(|r| r.mod_id.as_str()).unwrap_or("");
        let entry = format!("{}|{}", name, last);
        for b in entry.bytes().chain(std::iter::once(b'\n')) {
            h ^= b as u64;
            h = h.wrapping_mul(0x100000001b3);
        }
    }
    format!("{:016x}", h)
}

/// Asks the Studio Link to look at these functions in the running game (only when the Lua
/// folder exists: the game made it). Rewritten only when the list changes.
pub fn write_probe(user_zomboid_dir: &Path, shared: &[SharedFunction]) {
    let lua_dir = user_zomboid_dir.join("Lua");
    if !lua_dir.is_dir() {
        return;
    }
    let mut text = format!("list={}\n", if shared.is_empty() { String::new() } else { list_key(shared) });
    for name in probe_names(shared) {
        text.push_str(&format!("fn={}\n", name));
    }
    let path = lua_dir.join(PROBE_FILE);
    if std::fs::read_to_string(&path).is_ok_and(|now| now == text) {
        return;
    }
    let _ = crate::load_order::ini_parser::atomic_write(&path, text);
}

/// One reply of the Link: its list key, when, and each function's file and line.
fn read_reply(path: &Path) -> Option<(String, u64, HashMap<String, (String, u32)>)> {
    let text = std::fs::read_to_string(path).ok()?;
    let (mut key, mut at, mut found) = (String::new(), 0u64, HashMap::new());
    for line in text.lines() {
        if let Some(v) = line.strip_prefix("list=") {
            key = v.trim().to_string();
        } else if let Some(v) = line.strip_prefix("at=") {
            at = v.trim().parse().unwrap_or(0);
        } else if let Some(v) = line.strip_prefix("fn=") {
            let mut parts = v.split('\t');
            let (Some(name), Some(file), line_no) = (parts.next(), parts.next(), parts.next()) else { continue };
            if file != "-" {
                found.insert(name.to_string(), (file.to_string(), line_no.and_then(|l| l.trim().parse().ok()).unwrap_or(0)));
            }
        }
    }
    Some((key, at, found))
}

fn norm(path: &str) -> String {
    path.replace('\\', "/").to_lowercase()
}

/// The mod a file the game reported belongs to: a redefinition of the function in that file (by
/// its mod's folder, else by its path under media/lua), else any active mod whose folder holds it.
fn mod_of_file(file: &str, f: &SharedFunction, roots: &HashMap<String, String>) -> Option<String> {
    let n = norm(file);
    let under_root = |id: &str| roots.get(id).is_some_and(|root| n.starts_with(root.as_str()) || n.contains(root.as_str()));
    let rel_matches = |r: &Redefinition| n.ends_with(&format!("/media/lua/{}", norm(&r.file))) || n.ends_with(&norm(&r.file));
    if let Some(r) = f.redefinitions.iter().rev().find(|r| rel_matches(r) && under_root(&r.mod_id)) {
        return Some(r.mod_id.clone());
    }
    if let Some(r) = f.redefinitions.iter().rev().find(|r| rel_matches(r)) {
        return Some(r.mod_id.clone());
    }
    roots.iter().filter(|(_, root)| n.contains(root.as_str())).max_by_key(|(_, root)| root.len()).map(|(id, _)| id.clone())
}

/// Adds what the running game had, from the Link's latest reply for this very list: a save's
/// (all handlers have run) over the menu's.
pub fn attach_checks(user_zomboid_dir: &Path, catalog: &crate::catalog::ModCatalog, shared: &mut [SharedFunction]) {
    if shared.is_empty() {
        return;
    }
    let key = list_key(shared);
    let lua_dir = user_zomboid_dir.join("Lua");
    let replies: Vec<(&str, (String, u64, HashMap<String, (String, u32)>))> = [("game", PROBE_GAME_FILE), ("menu", PROBE_MENU_FILE)]
        .into_iter()
        .filter_map(|(when, name)| read_reply(&lua_dir.join(name)).map(|r| (when, r)))
        .filter(|(_, r)| r.0 == key)
        .collect();
    if replies.is_empty() {
        return;
    }
    let mut roots: HashMap<String, String> = HashMap::new();
    for f in shared.iter() {
        for r in &f.redefinitions {
            if let Some(m) = catalog.resolve(&r.mod_id) {
                roots.insert(r.mod_id.clone(), norm(&m.root_dir));
            }
        }
    }
    // The game may have a version from a mod that redefines it in a way the files do not show
    // (a loop over tables): that mod is named by its folder all the same.
    for m in catalog.mods.iter().filter(|m| m.shadowed_by.is_none()) {
        roots.entry(m.id.clone()).or_insert_with(|| norm(&m.root_dir));
    }
    for f in shared.iter_mut() {
        for (when, (_, at, found)) in &replies {
            let Some((file, line)) = found.get(&f.function) else { continue };
            let expected = f.redefinitions.iter().rev().find(|r| *when == "game" || runs_by_menu(r)).map(|r| r.mod_id.clone());
            let mod_id = mod_of_file(file, f, &roots);
            let agrees = mod_id == expected;
            f.checked = Some(Checked { when: when.to_string(), mod_id, file: file.clone(), line: *line, agrees, at: *at });
            break;
        }
    }
}

#[cfg(test)]
mod live {
    /// The shared functions of a real list. Ignored: needs the PC's game and mods.
    /// PZMS_PATHS=<pz>|<workshop>|<zomboid> PZMS_ORDER=<json array file>
    #[test]
    #[ignore]
    fn shared_functions_of_a_real_list() {
        let (Ok(paths), Ok(order)) = (std::env::var("PZMS_PATHS"), std::env::var("PZMS_ORDER")) else { return };
        let p: Vec<&str> = paths.split('|').collect();
        let active: Vec<String> = serde_json::from_str(&std::fs::read_to_string(order).unwrap()).unwrap();
        let catalog = crate::catalog::build_catalog(p[1], p[2], None);
        let t = std::time::Instant::now();
        let shared = super::shared_functions(std::path::Path::new(p[0]), &catalog, &active);
        eprintln!("{} mods, {} shared functions in {} ms", active.len(), shared.len(), t.elapsed().as_millis());
        let t = std::time::Instant::now();
        let again = super::shared_functions(std::path::Path::new(p[0]), &catalog, &active);
        eprintln!("again: {} in {} ms", again.len(), t.elapsed().as_millis());
        for f in shared.iter().filter(|f| f.winner.is_some()) {
            eprintln!("CUT {}: {} cuts {:?} (fix by moving before: {:?})", f.function, f.winner.as_deref().unwrap_or(""), f.cut, f.fixed_by_moving_before);
            for r in &f.redefinitions {
                eprintln!("    {} {}:{} keeps={} deferred={} gamefile={}", r.mod_id, r.file, r.line, r.keeps_original, r.deferred, r.replaces_game_file);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn defs(text: &str) -> Vec<(String, bool)> {
        scan(text).defs.into_iter().map(|d| (d.0, d.2)).collect()
    }

    #[test]
    fn definitions_in_every_shape() {
        let s = scan(
            "local old = ISFoo.bar\nfunction ISFoo:bar(x) return old(self, x) end\nISFoo.baz = function() end\n\
             local function mine() end\nISFoo.qux = mine\nISFoo.call(1)\nlocal t = { a = ISFoo.nope }\n",
        );
        assert_eq!(s.defs.iter().map(|d| d.0.as_str()).collect::<Vec<_>>(), ["ISFoo.bar", "ISFoo.baz", "ISFoo.qux"]);
        assert!(s.reads.contains("ISFoo.bar"), "kept with local old = ...");
        assert!(!s.reads.contains("ISFoo.call"), "a call is not keeping");
        assert!(s.locals.contains("old") && s.locals.contains("mine"));
    }

    #[test]
    fn a_wrap_inside_a_function_is_deferred_and_x_or_original_keeps() {
        let text = "function C.install()\n  local update = ISVehicleMechanics.update\n  function ISVehicleMechanics:update() return update(self) end\nend\n\
                    C.orig = C.orig or ISWorldObjectContextMenu.createMenu\nISWorldObjectContextMenu.createMenu = function(...) return C.orig(...) end\n";
        assert_eq!(
            defs(text),
            [("C.install".to_string(), false), ("ISVehicleMechanics.update".to_string(), true), ("ISWorldObjectContextMenu.createMenu".to_string(), false)]
        );
        let s = scan(text);
        assert!(s.reads.contains("ISWorldObjectContextMenu.createMenu") && s.reads.contains("ISVehicleMechanics.update"));
    }

    #[test]
    fn a_fallback_for_a_missing_game_table_is_no_replacement() {
        let text = "if not ISBaseTimedAction then
  ISBaseTimedAction = {}
  function ISBaseTimedAction:isValid() return true end
end
                    if ISFoo.bar == nil then ISFoo.bar = function() end end
function ISBaseTimedAction:start() end
";
        assert_eq!(defs(text), [("ISBaseTimedAction.start".to_string(), false)]);
    }

    #[test]
    fn strings_comments_and_loops_do_not_fool_it() {
        let text = "-- function ISFoo.a() end\nlocal s = \"ISFoo.b = function\"\nfor i = 1, 3 do\n  if i then end\nend\nISFoo.c = function() end\n";
        assert_eq!(defs(text), [("ISFoo.c".to_string(), false)], "the loop's do opened nothing more: c is at file level");
    }

    fn r(mod_id: &str, keeps: bool, deferred: bool) -> Redefinition {
        Redefinition { mod_id: mod_id.into(), mod_name: mod_id.into(), file: "client/x.lua".into(), line: 1, keeps_original: keeps, deferred, replaces_game_file: false, event: None, reset: false, conditional: false }
    }

    #[test]
    fn chaining_mods_cut_no_one() {
        let v = verdict("F".into(), vec![(0, r("A", true, false)), (1, r("B", true, false))]);
        assert_eq!((v.winner, v.cut), (None, vec![]));
    }

    #[test]
    fn a_replacement_after_others_cuts_them_and_the_order_can_fix_it() {
        let v = verdict("F".into(), vec![(0, r("A", true, false)), (3, r("B", false, false))]);
        assert_eq!((v.winner.as_deref(), v.cut.clone(), v.fixed_by_moving_before.as_deref()), (Some("B"), vec!["A".to_string()], Some("A")));
        // Two replacing: no order makes both run.
        let both = verdict("F".into(), vec![(0, r("A", false, false)), (3, r("B", false, false))]);
        assert_eq!((both.cut, both.fixed_by_moving_before), (vec!["A".to_string()], None));
    }

    #[test]
    fn a_mod_that_wraps_again_later_is_not_cut() {
        let v = verdict("F".into(), vec![(0, r("A", true, false)), (3, r("B", false, false)), (0, r("A", true, true))]);
        assert_eq!(v.winner, None, "A wraps B's version at OnGameStart");
    }

    #[test]
    fn a_replacement_first_and_a_deferred_wrap_after_is_fine() {
        // tsarslib replaces at file time; BlackHawk wraps it at OnGameStart: both run.
        let v = verdict("ISVehicleMechanics.update".into(), vec![(10, r("tsarslib", false, false)), (5, r("BlackHawk", true, true))]);
        assert_eq!(v.winner, None);
    }

    #[test]
    fn a_definition_in_an_event_handler_knows_its_event() {
        let s = scan("Events.OnGameStart.Add(function()\n  ISFoo.bar = function() end\nend)\nfunction M.install()\n  ISFoo.baz = function() end\nend\n");
        let events: Vec<(String, bool, Option<String>)> = s.defs.into_iter().map(|d| (d.0, d.2, d.3)).collect();
        assert_eq!(
            events,
            [
                ("ISFoo.bar".to_string(), true, Some("OnGameStart".to_string())),
                ("M.install".to_string(), false, None),
                ("ISFoo.baz".to_string(), true, None),
            ]
        );
    }

    #[test]
    fn a_table_made_new_is_a_reset_but_a_fallback_or_a_local_is_not() {
        let s = scan("ISFoo = {}\nISBar = ISBar or {}\nlocal Mine = {}\nif not ISBaz then ISBaz = {} end\n");
        assert_eq!(s.resets, [("ISFoo".to_string(), 1)]);
    }

    /// Measured on a 340-mod list (2026-11-18): `button.title = label`, `option.toolTip = tip`,
    /// `def.bodyModel = BODY` were read as functions of global tables `button`/`option`/`def`,
    /// and `target = { x = 1 }` inside a table as a global table made new.
    #[test]
    fn a_field_set_to_a_value_a_parameter_and_a_table_key_are_not_definitions() {
        let s = scan(
            "local function base(def)\n  def.bodyModel = BODY\nend\n\
             function MyMod.tip(option, player)\n  local tip = {}\n  option.toolTip = tip\nend\n\
             local function mine() end\nISFoo.qux = mine\nISFoo.later = elsewhere\n\
             return { anchors = { a = 1 }, target = { x = 12766 } }\n",
        );
        assert_eq!(s.defs.iter().map(|d| d.0.as_str()).collect::<Vec<_>>(), ["MyMod.tip", "ISFoo.qux"], "{:?}", s.defs);
        assert!(s.locals.contains("def") && s.locals.contains("option"), "parameters are locals");
        assert!(s.resets.is_empty() && !s.globals.contains("target") && !s.globals.contains("anchors"), "{:?} {:?}", s.resets, s.globals);
        // `ISFoo.later = elsewhere` waits for the mod: a definition when another file makes it.
        // `def.bodyModel = BODY` too (a parameter's field, left out by the mod's pass).
        assert_eq!(s.aliases.iter().map(|(d, v)| (d.0.as_str(), v.as_str())).collect::<Vec<_>>(), [("def.bodyModel", "BODY"), ("ISFoo.later", "elsewhere")]);
    }

    /// Two mods with a Lua file at the same path: the game runs only the copy of the one later in
    /// the list (it logs `mod "B" overrides media/lua/...`), so the first one's functions are not
    /// cut off by the second's, they never load. On a 340-mod list, 476 of 510 "cut" functions
    /// were this (variants of one mod shipping the same files).
    #[test]
    fn a_file_another_mod_replaces_at_its_path_never_runs() {
        let root = std::env::temp_dir().join(format!("pzms-overrides-paths-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let write = |rel: &str, text: &str| {
            let p = root.join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, text).unwrap();
        };
        write("game/media/lua/client/ISUI/ISFoo.lua", "ISFoo = {}\nfunction ISFoo.bar() end\n");
        for (id, file, text) in [
            ("ModA", "Shared/Same.lua", "function ISFoo.bar() return 1 end\n"),
            ("ModB", "shared/same.lua", "function ISFoo.bar() return 1 end\n"),
            ("ModC", "Other.lua", "function ISFoo.bar() return 2 end\n"),
        ] {
            write(&format!("zomboid/mods/{}/mod.info", id), &format!("name={}\nid={}\n", id, id));
            write(&format!("zomboid/mods/{}/media/lua/client/{}", id, file), text);
        }
        std::fs::create_dir_all(root.join("workshop")).unwrap();
        let catalog = crate::catalog::build_catalog(&root.join("workshop").to_string_lossy(), &root.join("zomboid").to_string_lossy(), Some(41_078));
        let shared = |list: &[&str]| shared_functions(&root.join("game"), &catalog, &list.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        let only_same_path = shared(&["ModA", "ModB"]);
        let with_other = shared(&["ModA", "ModB", "ModC"]);
        let _ = std::fs::remove_dir_all(&root);
        assert!(only_same_path.is_empty(), "{:?}", only_same_path);
        assert_eq!(with_other.len(), 1);
        let mods: Vec<&str> = with_other[0].redefinitions.iter().map(|r| r.mod_id.as_str()).collect();
        assert_eq!(mods, ["ModB", "ModC"], "ModA's copy never runs");
        assert_eq!(with_other[0].cut, ["ModB"]);
    }

    /// Measured on two players' collections (2026-11-18): an alert library replaces
    /// `MainScreen.onEnterFromGame` and an addon that requires it wraps it, but the list had the
    /// addon first: read in list order the library "cut" the addon, while the game loads a
    /// requirement first and the addon wraps it. And a requirement that is off is loaded by the
    /// game all the same: its replacement after another mod's wrap was not seen at all.
    #[test]
    fn files_run_in_the_order_the_game_loads_the_mods() {
        let root = std::env::temp_dir().join(format!("pzms-overrides-order-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let write = |rel: &str, text: &str| {
            let p = root.join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, text).unwrap();
        };
        write("game/media/lua/client/ISUI/ISFoo.lua", "ISFoo = {}\nfunction ISFoo.bar() end\nfunction ISFoo.baz() end\n");
        for (id, info, text) in [
            ("Alert", "", "function ISFoo:bar() end\n"),
            ("Addon", "require=\\Alert\n", "local old = ISFoo.bar\nfunction ISFoo:bar() old(self) end\n"),
            ("Wrapper", "", "local old = ISFoo.baz\nfunction ISFoo.baz() old() end\n"),
            ("Framework", "", "function ISFoo.baz() end\n"),
            ("UsesFramework", "require=\\Framework\n", "local x = 1\n"),
        ] {
            write(&format!("zomboid/mods/{}/mod.info", id), &format!("name={}\nid={}\n{}", id, id, info));
            write(&format!("zomboid/mods/{}/media/lua/client/{}.lua", id, id), text);
        }
        std::fs::create_dir_all(root.join("workshop")).unwrap();
        let catalog = crate::catalog::build_catalog(&root.join("workshop").to_string_lossy(), &root.join("zomboid").to_string_lossy(), Some(41_078));
        let list = |ids: &[&str]| ids.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        // Addon listed before the library it requires; Framework off but required by a mod on.
        let shared = shared_functions(&root.join("game"), &catalog, &list(&["Addon", "Alert", "Wrapper", "UsesFramework"]));
        let _ = std::fs::remove_dir_all(&root);
        let bar = shared.iter().find(|f| f.function == "ISFoo.bar").expect("both redefine bar");
        assert_eq!(bar.redefinitions.iter().map(|r| r.mod_id.as_str()).collect::<Vec<_>>(), ["Alert", "Addon"], "the requirement runs first");
        assert_eq!(bar.winner, None, "the addon wraps the library's version: no one is cut");
        let baz = shared.iter().find(|f| f.function == "ISFoo.baz").expect("the requirement that is off is loaded too");
        assert_eq!((baz.winner.as_deref(), baz.cut.clone()), (Some("Framework"), vec!["Wrapper".to_string()]));
    }

    /// What makes a cut certain (Mod Health's certainty chip): measured on three collections
    /// (2026-11-18), every cut made at file load was confirmed by reading the code; a replacement
    /// made in a handler or install() runs only under its own conditions (a sandbox option), and a
    /// file can stop before its definition (`if getActivatedMods():contains("X") then return end`).
    #[test]
    fn a_cut_made_while_the_files_load_is_certain_and_a_conditional_one_is_not() {
        let s = scan("if getActivatedMods():contains(\"x\") then return end\nfunction ISFoo:bar() end\n");
        assert_eq!(s.early_return, Some(1));
        assert_eq!(scan("function ISFoo:bar() return 1 end\nreturn ISFoo\n").early_return, None, "a return inside a function or at the very end is no early stop");

        let file_time = verdict("F".into(), vec![(0, r("A", true, false)), (3, r("B", false, false))]);
        assert!(cut_is_certain(&file_time));
        let in_handler = verdict("F".into(), vec![(0, r("A", true, false)), (3, r("B", false, true))]);
        assert_eq!(in_handler.cut, ["A"]);
        assert!(!cut_is_certain(&in_handler), "the replacement runs from a function: whether it does is the game's to say");
        let mut stops = r("B", false, false);
        stops.conditional = true;
        let early = verdict("F".into(), vec![(0, r("A", true, false)), (3, stops)]);
        assert!(!cut_is_certain(&early));
        assert!(!cut_is_certain(&verdict("F".into(), vec![(0, r("A", true, false)), (1, r("B", true, false))])), "no cut, nothing certain");
    }

    /// Seen in the running game (Studio Link, 389-mod collection, 2026-11-18): 39 of 140 functions
    /// were not what the files suggested at the main menu. Most were wraps made inside a named
    /// function the file runs itself (`applyPatches()` at its end) or gives to an event by name
    /// (`Events.OnGameBoot.Add(install)`), taken for "later than everything"; and files of the
    /// server/ folder, which load only with a save, taken as loaded by the menu.
    #[test]
    fn a_named_install_runs_when_the_file_runs_or_gives_it_to_an_event() {
        let s = scan(
            "local function applyPatches()\n  local old = ISFoo.a\n  function ISFoo.a() old() end\nend\napplyPatches()\n\
             local function install()\n  ISFoo.b = function() end\nend\nEvents.OnGameBoot.Add(install)\n\
             local later = function()\n  ISFoo.c = function() end\nend\nM.hook = later\n\
             local function both()\n  ISFoo.d = function() end\nend\nboth()\nEvents.OnGameStart.Add(both)\n\
             local function mine() end\nlocal function aliased()\n  ISFoo.e = mine\nend\naliased()\n",
        );
        let got: Vec<(String, bool, Option<String>)> = s.defs.into_iter().map(|d| (d.0, d.2, d.3)).collect();
        assert_eq!(
            got,
            [
                ("ISFoo.a".to_string(), false, None),
                ("ISFoo.b".to_string(), true, Some("OnGameBoot".to_string())),
                ("ISFoo.c".to_string(), true, None),
                ("M.hook".to_string(), false, None),
                ("ISFoo.d".to_string(), false, None),
                ("ISFoo.e".to_string(), false, None),
            ]
        );
        let mut server = r("A", true, false);
        server.file = "server/A.lua".into();
        assert!(!runs_by_menu(&server), "server/ files load with a save");
        assert!(runs_by_menu(&r("B", true, false)));
    }

    /// Measured on two players' collections (2026-11-18): a firearms mod wraps seven of the game's
    /// timed actions through one function given each table (`wrapAction(ISReloadWeaponAction)`),
    /// and none of those wraps were seen.
    #[test]
    fn a_function_given_a_table_at_file_level_defines_on_that_table() {
        let s = scan(
            "local function wrapAction(class, onFinish)\n  local origStart = class.start\n  if origStart then\n    class.start = function(self) return origStart(self) end\n  end\n  class.stop = function(self) end\nend\n\
             wrapAction(ISReload)\nwrapAction(ISRack, function(gun) if gun then x() end end)\nwrapAction(makeOne())\n\
             local function later(t)\n  t.perform = function() end\nend\nEvents.OnGameStart.Add(function() later(ISLate) end)\n",
        );
        let got: Vec<(String, u32, bool)> = s.defs.iter().filter(|d| d.0.starts_with("IS")).map(|d| (d.0.clone(), d.1, d.2)).collect();
        assert_eq!(
            got,
            [
                ("ISReload.start".to_string(), 4, false),
                ("ISRack.start".to_string(), 4, false),
                ("ISReload.stop".to_string(), 6, false),
                ("ISRack.stop".to_string(), 6, false),
            ],
            "a call with something else than a table, or from inside a function, gives nothing"
        );
        assert!(s.reads.contains("ISReload.start") && s.reads.contains("ISRack.start"), "it keeps the original start");
        assert!(!s.reads.contains("ISReload.stop"), "stop is replaced outright");
        assert!(s.locals.contains("class"), "the parameter's own definitions stay a local's");
    }

    #[test]
    fn handlers_run_in_the_order_of_their_events() {
        let mut boot = r("A", false, true);
        boot.event = Some("OnGameBoot".into());
        let mut start = r("B", false, true);
        start.event = Some("OnGameStart".into());
        let file = r("C", true, false);
        // B (OnGameStart) runs after A (OnGameBoot), which runs after every file: B wins, A and C are cut.
        let mut defs = vec![(0, start.clone()), (1, boot.clone()), (2, file.clone())];
        defs.sort_by(|a, b| run_order(&a.1, a.0).cmp(&run_order(&b.1, b.0)));
        assert_eq!(defs.iter().map(|d| d.1.mod_id.as_str()).collect::<Vec<_>>(), ["C", "A", "B"]);
        assert!(runs_by_menu(&boot) && !runs_by_menu(&start));
    }

    #[test]
    fn what_the_game_reported_is_matched_to_its_mod() {
        let mut a = r("A", true, false);
        a.file = "client/ISUI/ISFoo.lua".into();
        let mut b = r("B", false, false);
        b.file = "client/B/Patch.lua".into();
        let f = verdict("ISFoo.bar".into(), vec![(0, a), (1, b)]);
        let roots: HashMap<String, String> = [("A".to_string(), "c:/mods/a".to_string()), ("B".to_string(), "c:/mods/b".to_string())].into();
        assert_eq!(mod_of_file(r"C:\mods\B\42\media\lua\client\B\Patch.lua", &f, &roots).as_deref(), Some("B"));
        assert_eq!(mod_of_file("media/lua/client/ISUI/ISFoo.lua", &f, &roots).as_deref(), Some("A"));
        assert_eq!(mod_of_file("D:/game/media/lua/client/Other.lua", &f, &roots), None);
    }
}
